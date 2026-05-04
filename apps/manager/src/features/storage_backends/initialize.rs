//! Explicit, type-to-confirm endpoint that runs the destructive
//! one-time `pvcreate` + `vgcreate` setup for an `iscsi_lvm` backend.
//!
//! This is intentionally separated from the `Add Backend` POST so an
//! operator pointing at the wrong LUN doesn't wipe its contents on
//! every config change. The UI puts a type-to-confirm dialog in front
//! of this route (Task 15).

use crate::features::storage_backends::repo::StorageBackendRepository;
use crate::AppState;
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use uuid::Uuid;

const REQUIRED_CONFIRM_PHRASE: &str = "I understand this wipes the LUN";

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct InitializeReq {
    pub confirm: String,
}

#[utoipa::path(
    post,
    path = "/v1/storage_backends/{id}/initialize",
    params(("id" = Uuid, Path, description = "Storage backend ID")),
    request_body = InitializeReq,
    responses(
        (status = 204, description = "VG initialized"),
        (status = 400, description = "Missing or wrong confirm phrase"),
        (status = 404, description = "Backend not found"),
        (status = 409, description = "Wrong backend kind"),
        (status = 422, description = "Agent returned an error (LUN already has different VG, etc.)"),
    ),
    tag = "StorageBackends",
)]
pub async fn initialize(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<InitializeReq>,
) -> impl IntoResponse {
    if req.confirm != REQUIRED_CONFIRM_PHRASE {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!(
                    "missing or wrong confirm phrase; expected exactly: {REQUIRED_CONFIRM_PHRASE:?}"
                )
            })),
        )
            .into_response();
    }

    let repo = StorageBackendRepository::new(st.db.clone());
    let row = match repo.get(id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "not found"})),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!(?e, "initialize: db lookup");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response();
        }
    };
    if row.kind != "iscsi_lvm" {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "initialize is only valid for iscsi_lvm backends"
            })),
        )
            .into_response();
    }

    // Decode IscsiLvmConfig from row.config_json. Fill agent_url default from
    // most-recently-seen host, mirroring registry.rs / probe_iscsi_lvm in health.rs.
    use crate::features::storage::backends::iscsi_lvm::IscsiLvmConfig;
    let mut cfg: IscsiLvmConfig = match serde_json::from_value(row.config_json.clone()) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("decode iscsi_lvm config: {e}")
                })),
            )
                .into_response();
        }
    };
    if cfg.agent_url.is_none() {
        cfg.agent_url = sqlx::query_scalar::<_, String>(
            "SELECT addr FROM host ORDER BY last_seen_at DESC LIMIT 1",
        )
        .fetch_optional(&st.db)
        .await
        .ok()
        .flatten();
    }
    let agent_url_raw = match cfg.agent_url.as_deref() {
        Some(u) if !u.is_empty() => u.to_string(),
        _ => {
            return (
                StatusCode::FAILED_DEPENDENCY,
                Json(serde_json::json!({
                    "error": "no agent_url available; ensure an agent is registered"
                })),
            )
                .into_response();
        }
    };

    // Canonicalize: prepend http:// when no scheme, strip trailing slash —
    // matches probe_iscsi_lvm in health.rs.
    let base = {
        let with_scheme =
            if agent_url_raw.starts_with("http://") || agent_url_raw.starts_with("https://") {
                agent_url_raw.clone()
            } else {
                format!("http://{agent_url_raw}")
            };
        with_scheme.trim_end_matches('/').to_string()
    };

    // Call agent's /v1/storage/iscsi_lvm/init_vg.
    let client = match reqwest::Client::builder()
        // pvcreate + vgcreate can take a few seconds on a real LUN.
        .timeout(std::time::Duration::from_secs(60))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("reqwest builder: {e}")})),
            )
                .into_response();
        }
    };
    let body = serde_json::json!({
        "iqn": cfg.iqn,
        "portal": cfg.portal,
        "lun": cfg.lun,
        "vg_name": cfg.vg_name,
    });
    match client
        .post(format!("{base}/v1/storage/iscsi_lvm/init_vg"))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            tracing::info!(backend_id=%id, vg=%cfg.vg_name, "iscsi_lvm VG initialized");
            (StatusCode::NO_CONTENT, ()).into_response()
        }
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "error": format!("agent init_vg failed (HTTP {status}): {body}")
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "error": format!("agent init_vg request failed: {e}")
            })),
        )
            .into_response(),
    }
}
