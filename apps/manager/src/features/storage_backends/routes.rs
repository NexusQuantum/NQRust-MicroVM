use crate::features::storage::config::{validate, RawBackendEntry};
use crate::features::storage_backends::repo::{StorageBackendRepository, StorageBackendRow};
use crate::AppState;
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Extension, Json};
use nexus_types::{Capabilities, StorageBackend};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use uuid::Uuid;

fn row_to_wire(row: StorageBackendRow) -> Result<StorageBackend, StatusCode> {
    let kind: nexus_types::BackendKind =
        serde_json::from_value(serde_json::Value::String(row.kind.clone()))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let capabilities: Capabilities = match serde_json::from_value(row.capabilities_json) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                "storage_backend '{}' has malformed capabilities_json; using default: {e}",
                row.name
            );
            Capabilities::default()
        }
    };
    Ok(StorageBackend {
        id: row.id,
        name: row.name,
        kind,
        capabilities,
        is_default: row.is_default,
        created_at: row.created_at,
        deleted_at: row.deleted_at,
    })
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct StorageBackendListResponse {
    pub items: Vec<StorageBackend>,
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends",
    responses(
        (status = 200, body = StorageBackendListResponse),
    ),
    tag = "StorageBackends",
)]
pub async fn list(Extension(st): Extension<AppState>) -> impl IntoResponse {
    let repo = StorageBackendRepository::new(st.db.clone());
    match repo.list_active().await {
        Ok(rows) => {
            let mut items = Vec::with_capacity(rows.len());
            for r in rows {
                match row_to_wire(r) {
                    Ok(w) => items.push(w),
                    Err(s) => {
                        return (s, Json(serde_json::json!({"error": "row deserialization"})))
                            .into_response()
                    }
                }
            }
            (StatusCode::OK, Json(StorageBackendListResponse { items })).into_response()
        }
        Err(e) => {
            tracing::error!("storage_backends list failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response()
        }
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateStorageBackendReq {
    pub name: String,
    pub kind: nexus_storage::BackendKind,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub config: JsonValue,
    /// SMB-only: send to agent's /set_credentials route; never persisted in DB.
    #[serde(default)]
    pub password: Option<String>,
}

#[utoipa::path(
    post,
    path = "/v1/storage_backends",
    request_body = CreateStorageBackendReq,
    responses(
        (status = 201, body = StorageBackend),
        (status = 400, description = "Validation failed"),
        (status = 409, description = "Backend with this name already exists"),
    ),
    tag = "StorageBackends",
)]
pub async fn create(
    Extension(st): Extension<AppState>,
    Json(req): Json<CreateStorageBackendReq>,
) -> impl IntoResponse {
    let validated = match validate(RawBackendEntry {
        name: req.name.clone(),
        kind: req.kind,
        is_default: req.is_default,
        config: req.config,
    }) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };
    let capabilities_json = match serde_json::to_value(validated.capabilities) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("encode capabilities: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "encode capabilities"})),
            )
                .into_response();
        }
    };
    let repo = StorageBackendRepository::new(st.db.clone());
    // Idempotent on (name). A second POST with the same name updates
    // the existing row in place — that's the behaviour operators
    // expect when iterating on a backend's config in the UI form.
    let row = match repo
        .upsert(
            &validated.name,
            validated.kind.as_db_str(),
            &validated.config,
            &capabilities_json,
            validated.is_default,
            "ui",
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("storage_backends create failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response();
        }
    };

    // For NFS: eagerly probe the agent-side mount so the operator sees
    // "your share isn't reachable" right here in the create response,
    // not later on first VM-create. On failure, undo the upsert so the
    // table doesn't show a broken backend.
    if validated.kind == nexus_storage::BackendKind::Nfs {
        if let Err(e) = probe_nfs_backend(&st, &row).await {
            let _ = repo.soft_delete_by_name(&row.name).await;
            let chain: Vec<String> = e.chain().map(|c| c.to_string()).collect();
            let detail = chain.join(" -> ");
            tracing::warn!(backend = %row.name, error = ?e, "nfs backend probe failed; row rolled back");
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({"error": format!("nfs mount probe failed: {detail}")})),
            )
                .into_response();
        }
    }

    // SMB: deliver credentials to the agent (if any) then probe. On failure,
    // roll back the row and best-effort clear any cred file we just wrote so
    // the agent doesn't keep stale creds for a backend that no longer exists.
    if validated.kind == nexus_storage::BackendKind::Smb {
        if let Err(e) = probe_smb_backend(&st, &row, req.password.as_deref()).await {
            let _ = repo.soft_delete_by_name(&row.name).await;
            let _ = clear_smb_credentials_on_agent(&st, &row).await;
            let chain: Vec<String> = e.chain().map(|c| c.to_string()).collect();
            let detail = chain.join(" -> ");
            tracing::warn!(backend = %row.name, error = ?e, "smb backend probe failed; row rolled back");
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({"error": format!("smb mount probe failed: {detail}")})),
            )
                .into_response();
        }
    }

    // Insert into the live registry so the backend is immediately usable
    // for VM-create without a manager restart. Best-effort: a failure here
    // is logged but doesn't roll back the DB row — operators can still see
    // the backend in the table; it'll come online on next manager restart.
    if let Err(e) = st.registry.add(&row).await {
        tracing::warn!(
            backend = %row.name,
            error = %e,
            "create succeeded but live-registry insert failed (next restart will pick it up)"
        );
    }

    match row_to_wire(row) {
        Ok(w) => (StatusCode::CREATED, Json(w)).into_response(),
        Err(s) => (s, Json(serde_json::json!({"error": "row deserialization"}))).into_response(),
    }
}

async fn probe_nfs_backend(st: &AppState, row: &StorageBackendRow) -> anyhow::Result<()> {
    use crate::features::storage::backends::nfs::{NfsConfig, NfsControlPlaneBackend};
    use anyhow::Context;
    use nexus_storage::{BackendInstanceId, ControlPlaneBackend};

    let mut cfg: NfsConfig =
        serde_json::from_value(row.config_json.clone()).context("decode nfs config for probe")?;
    if cfg.agent_url.is_none() {
        let default_agent_url: Option<String> =
            sqlx::query_scalar("SELECT addr FROM host ORDER BY last_seen_at DESC LIMIT 1")
                .fetch_optional(&st.db)
                .await
                .context("looking up default agent for nfs probe")?;
        cfg.agent_url = default_agent_url;
    }
    let backend = NfsControlPlaneBackend {
        id: BackendInstanceId(row.id),
        config: cfg,
    };
    backend.probe().await.context("nfs mount probe")?;
    Ok(())
}

async fn probe_smb_backend(
    st: &AppState,
    row: &StorageBackendRow,
    password: Option<&str>,
) -> anyhow::Result<()> {
    use crate::features::storage::backends::smb::{SmbConfig, SmbControlPlaneBackend};
    use anyhow::Context;
    use nexus_storage::{BackendInstanceId, ControlPlaneBackend};

    let mut cfg: SmbConfig =
        serde_json::from_value(row.config_json.clone()).context("decode smb config for probe")?;
    if cfg.agent_url.is_none() {
        let default_agent_url: Option<String> =
            sqlx::query_scalar("SELECT addr FROM host ORDER BY last_seen_at DESC LIMIT 1")
                .fetch_optional(&st.db)
                .await
                .context("looking up default agent for smb probe")?;
        cfg.agent_url = default_agent_url;
    }
    let agent_url = cfg
        .agent_url
        .as_deref()
        .filter(|u| !u.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("smb backend has no agent_url and no registered agent host")
        })?
        .to_string();

    // If the share is authenticated (username present) AND we received a
    // password on this request, push the cred file to the agent before the
    // probe — otherwise the probe's mount.cifs will fail with EACCES.
    if let (Some(username), Some(password)) =
        (cfg.username.as_deref().filter(|u| !u.is_empty()), password)
    {
        set_smb_credentials_on_agent(
            &agent_url,
            row.id,
            username,
            password,
            cfg.domain.as_deref(),
        )
        .await
        .context("delivering smb credentials to agent")?;
    }

    let backend = SmbControlPlaneBackend {
        id: BackendInstanceId(row.id),
        config: cfg,
    };
    backend.probe().await.context("smb mount probe")?;
    Ok(())
}

async fn set_smb_credentials_on_agent(
    agent_url: &str,
    backend_id: uuid::Uuid,
    username: &str,
    password: &str,
    domain: Option<&str>,
) -> anyhow::Result<()> {
    let url = format!(
        "{}/v1/storage/smb/set_credentials",
        agent_url.trim_end_matches('/')
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "backend_id": backend_id,
            "username": username,
            "password": password,
            "domain": domain,
        }))
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("agent set_credentials failed: HTTP {status}: {body}");
    }
    Ok(())
}

/// Push (or rewrite) the per-backend SMB cred file on the agent. Used by
/// the UPDATE handler when the operator submits a new password without
/// re-probing the share. Errors propagate so the caller can decide whether
/// to surface them (we currently just log).
async fn refresh_smb_credentials_on_agent(
    st: &AppState,
    row: &StorageBackendRow,
    password: &str,
) -> anyhow::Result<()> {
    use anyhow::Context;
    let cfg: crate::features::storage::backends::smb::SmbConfig =
        serde_json::from_value(row.config_json.clone()).context("decode smb config for refresh")?;
    let username = cfg
        .username
        .as_deref()
        .filter(|u| !u.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("smb backend has no username; cannot store password without a user")
        })?
        .to_string();
    let agent_url = if let Some(u) = cfg.agent_url.clone().filter(|u| !u.is_empty()) {
        u
    } else {
        let fallback: Option<String> =
            sqlx::query_scalar("SELECT addr FROM host ORDER BY last_seen_at DESC LIMIT 1")
                .fetch_optional(&st.db)
                .await
                .context("looking up default agent for smb cred refresh")?;
        fallback
            .filter(|u| !u.is_empty())
            .ok_or_else(|| anyhow::anyhow!("no agent_url and no registered agent host"))?
    };
    set_smb_credentials_on_agent(
        &agent_url,
        row.id,
        &username,
        password,
        cfg.domain.as_deref(),
    )
    .await
    .context("delivering smb credentials to agent")?;
    Ok(())
}

/// Best-effort: tells the agent to remove the per-backend cred file. Used on
/// rollback after a failed probe and on backend delete. Swallows network
/// errors — the caller is already on a cleanup path.
async fn clear_smb_credentials_on_agent(
    st: &AppState,
    row: &StorageBackendRow,
) -> anyhow::Result<()> {
    let cfg: crate::features::storage::backends::smb::SmbConfig =
        serde_json::from_value(row.config_json.clone())?;
    let agent_url = if let Some(u) = cfg.agent_url.filter(|u| !u.is_empty()) {
        u
    } else {
        let fallback: Option<String> =
            sqlx::query_scalar("SELECT addr FROM host ORDER BY last_seen_at DESC LIMIT 1")
                .fetch_optional(&st.db)
                .await
                .ok()
                .flatten();
        match fallback {
            Some(u) if !u.is_empty() => u,
            _ => return Ok(()),
        }
    };
    let full_url = format!(
        "{}/v1/storage/smb/clear_credentials",
        agent_url.trim_end_matches('/')
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    let _ = client
        .post(&full_url)
        .json(&serde_json::json!({ "backend_id": row.id }))
        .send()
        .await;
    Ok(())
}

#[utoipa::path(
    delete,
    path = "/v1/storage_backends/{id}",
    params(("id" = Uuid, Path, description = "Storage backend ID")),
    responses(
        (status = 204, description = "Soft-deleted"),
        (status = 404),
        (status = 409, description = "Backend has live volumes"),
    ),
    tag = "StorageBackends",
)]
pub async fn delete(Extension(st): Extension<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
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
            tracing::error!("storage_backends get for delete failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response();
        }
    };
    // Refuse delete if any active volume references this backend.
    // A foreign-key cascade isn't right here — soft-deleting a backend
    // with live volumes attached would orphan them.
    let live_count: Result<i64, _> = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM volume WHERE backend_id = $1 AND status != 'deleted'"#,
    )
    .bind(row.id)
    .fetch_one(&st.db)
    .await;
    if let Ok(n) = live_count {
        if n > 0 {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": format!("backend has {n} live volume(s); delete or migrate them before removing the backend"),
                })),
            )
                .into_response();
        }
    }
    if let Err(e) = repo.soft_delete_by_name(&row.name).await {
        tracing::error!("storage_backends soft-delete failed: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "db"})),
        )
            .into_response();
    }
    // SMB: best-effort cred wipe on the agent so we don't leave an orphan
    // `/etc/nqrust/storage-creds/<id>.cred` behind after the backend is
    // gone. Agent unreachable / 5xx is fine — the file is small and the
    // backend_id can't be reused anyway.
    if row.kind == "smb" {
        if let Err(e) = clear_smb_credentials_on_agent(&st, &row).await {
            tracing::warn!(
                backend = %row.name,
                error = %e,
                "smb credential clear failed during delete (orphan cred file may remain)"
            );
        }
    }
    st.registry.remove(id);
    StatusCode::NO_CONTENT.into_response()
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends/{id}",
    params(("id" = Uuid, Path, description = "Storage backend ID")),
    responses(
        (status = 200, body = StorageBackend),
        (status = 404),
    ),
    tag = "StorageBackends",
)]
pub async fn get_one(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = StorageBackendRepository::new(st.db.clone());
    match repo.get(id).await {
        Ok(Some(row)) => match row_to_wire(row) {
            Ok(w) => (StatusCode::OK, Json(w)).into_response(),
            Err(s) => {
                (s, Json(serde_json::json!({"error": "row deserialization"}))).into_response()
            }
        },
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("storage_backends get failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response()
        }
    }
}

use crate::features::storage_backends::discovery::{
    discover_iscsi_targets, discover_nfs_exports, IscsiTarget, NfsExport,
};
use crate::features::storage_backends::health::check_backend_health;

#[derive(Debug, Deserialize)]
pub struct NfsScanQuery {
    pub server: String,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct NfsScanResponse {
    pub exports: Vec<NfsExportWire>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct NfsExportWire {
    pub path: String,
    pub allowed: String,
}

impl From<NfsExport> for NfsExportWire {
    fn from(e: NfsExport) -> Self {
        Self {
            path: e.path,
            allowed: e.allowed,
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends/scan/nfs",
    params(("server" = String, Query, description = "NFS server hostname or IP")),
    responses(
        (status = 200, body = NfsScanResponse),
        (status = 400, description = "server query param missing"),
        (status = 502, description = "Discovery failed (timeout, unreachable, command missing)"),
    ),
    tag = "StorageBackends",
)]
pub async fn scan_nfs(
    Extension(_st): Extension<AppState>,
    axum::extract::Query(q): axum::extract::Query<NfsScanQuery>,
) -> impl IntoResponse {
    if q.server.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "server query param is required"})),
        )
            .into_response();
    }
    match discover_nfs_exports(&q.server).await {
        Ok(exports) => (
            StatusCode::OK,
            Json(NfsScanResponse {
                exports: exports.into_iter().map(NfsExportWire::from).collect(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct IscsiScanQuery {
    pub portal: String,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct IscsiScanResponse {
    pub targets: Vec<IscsiTargetWire>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct IscsiTargetWire {
    pub portal: String,
    pub iqn: String,
}

impl From<IscsiTarget> for IscsiTargetWire {
    fn from(t: IscsiTarget) -> Self {
        Self {
            portal: t.portal,
            iqn: t.iqn,
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends/scan/iscsi",
    params(("portal" = String, Query, description = "iSCSI portal as host:port")),
    responses(
        (status = 200, body = IscsiScanResponse),
        (status = 400),
        (status = 502),
    ),
    tag = "StorageBackends",
)]
pub async fn scan_iscsi(
    Extension(_st): Extension<AppState>,
    axum::extract::Query(q): axum::extract::Query<IscsiScanQuery>,
) -> impl IntoResponse {
    if q.portal.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "portal query param is required"})),
        )
            .into_response();
    }
    match discover_iscsi_targets(&q.portal).await {
        Ok(targets) => (
            StatusCode::OK,
            Json(IscsiScanResponse {
                targets: targets.into_iter().map(IscsiTargetWire::from).collect(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends/{id}/health",
    params(("id" = Uuid, Path, description = "Storage backend ID")),
    responses((status = 200), (status = 404)),
    tag = "StorageBackends",
)]
pub async fn health(Extension(st): Extension<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let repo = StorageBackendRepository::new(st.db.clone());
    match repo.get(id).await {
        Ok(Some(row)) => {
            let default_agent_url: Option<String> =
                sqlx::query_scalar("SELECT addr FROM host ORDER BY last_seen_at DESC LIMIT 1")
                    .fetch_optional(&st.db)
                    .await
                    .ok()
                    .flatten();
            let h = check_backend_health(&row, default_agent_url.as_deref()).await;
            (StatusCode::OK, Json(h)).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("health get failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response()
        }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct StorageBackendConfigResponse {
    pub config: JsonValue,
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends/{id}/config",
    params(("id" = Uuid, Path, description = "Storage backend ID")),
    responses(
        (status = 200, body = StorageBackendConfigResponse),
        (status = 404),
    ),
    tag = "StorageBackends",
)]
/// Returns the raw config JSON for a backend so the UI can round-trip
/// it through the Edit dialog. Note: the config may contain sensitive
/// references like `api_key_env` (the env var name, not the key).
/// Future: redact known-sensitive fields before returning.
pub async fn get_config(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = StorageBackendRepository::new(st.db.clone());
    match repo.get(id).await {
        Ok(Some(row)) => (
            StatusCode::OK,
            Json(StorageBackendConfigResponse {
                config: row.config_json,
            }),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("storage_backends get_config failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    put,
    path = "/v1/storage_backends/{id}",
    params(("id" = Uuid, Path, description = "Storage backend ID")),
    request_body = CreateStorageBackendReq,
    responses(
        (status = 200, body = StorageBackend),
        (status = 400),
        (status = 404),
    ),
    tag = "StorageBackends",
)]
/// Update a storage backend's kind/config/capabilities/is_default.
/// Note: `name` is NOT modifiable through this endpoint — to rename
/// a backend, delete and re-add it. The PUT body uses the same shape
/// as POST `/v1/storage_backends` for consistency, but the `name`
/// field is ignored.
pub async fn update(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateStorageBackendReq>,
) -> impl IntoResponse {
    let password = req.password.clone();
    let validated = match validate(RawBackendEntry {
        name: req.name.clone(),
        kind: req.kind,
        is_default: req.is_default,
        config: req.config,
    }) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };
    let capabilities_json = match serde_json::to_value(validated.capabilities) {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "encode capabilities"})),
            )
                .into_response();
        }
    };
    let repo = StorageBackendRepository::new(st.db.clone());
    match repo
        .update(
            id,
            validated.kind.as_db_str(),
            &validated.config,
            &capabilities_json,
            validated.is_default,
        )
        .await
    {
        Ok(Some(row)) => {
            // SMB: refresh the cred file on the agent if the caller sent a
            // new password. We do NOT re-probe on update — the operator is
            // mid-edit and a transient share outage shouldn't block them
            // from saving. Failure here is logged but not fatal.
            if row.kind == "smb" {
                if let Some(pw) = password.as_deref() {
                    if let Err(e) = refresh_smb_credentials_on_agent(&st, &row, pw).await {
                        tracing::warn!(
                            backend = %row.name,
                            error = %e,
                            "smb credential refresh failed during update"
                        );
                    }
                }
            }
            match row_to_wire(row) {
                Ok(w) => (StatusCode::OK, Json(w)).into_response(),
                Err(s) => {
                    (s, Json(serde_json::json!({"error": "row deserialization"}))).into_response()
                }
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("storage_backends update failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response()
        }
    }
}
