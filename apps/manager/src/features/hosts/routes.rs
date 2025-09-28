use crate::AppState;
use axum::{extract::Path, http::StatusCode, Extension, Json};
use nexus_types::{
    HostHeartbeatRequest, HostPathParams, OkResponse, RegisterHostRequest, RegisterHostResponse,
};
use tracing::error;

#[utoipa::path(
    post,
    path = "/v1/hosts/register",
    request_body = RegisterHostRequest,
    responses(
        (status = 200, description = "Host registered", body = RegisterHostResponse),
        (status = 500, description = "Failed to register host"),
    ),
    tag = "Hosts"
)]
pub async fn register(
    Extension(st): Extension<AppState>,
    Json(req): Json<RegisterHostRequest>,
) -> Result<Json<RegisterHostResponse>, StatusCode> {
    let RegisterHostRequest {
        name,
        addr,
        capabilities,
    } = req;

    let row = st
        .hosts
        .register(&name, &addr, capabilities)
        .await
        .map_err(|err| {
            error!(?err, "failed to register host");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(RegisterHostResponse { id: row.id }))
}

#[utoipa::path(
    post,
    path = "/v1/hosts/{id}/heartbeat",
    params(HostPathParams),
    request_body = HostHeartbeatRequest,
    responses(
        (status = 200, description = "Heartbeat recorded", body = OkResponse),
        (status = 404, description = "Host not found"),
        (status = 500, description = "Failed to record heartbeat"),
    ),
    tag = "Hosts"
)]
pub async fn heartbeat(
    Extension(st): Extension<AppState>,
    Path(HostPathParams { id }): Path<HostPathParams>,
    Json(req): Json<HostHeartbeatRequest>,
) -> Result<Json<OkResponse>, StatusCode> {
    st.hosts
        .heartbeat(id, req.capabilities)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            other => {
                error!(error = ?other, "failed to record host heartbeat");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    Ok(Json(OkResponse::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::Path, Extension};
    use serde_json::json;

    #[sqlx::test(migrations = "./migrations")]
    async fn register_creates_host(pool: sqlx::PgPool) {
        let repo = crate::features::hosts::repo::HostRepository::new(pool.clone());
        let images =
            crate::features::images::repo::ImageRepository::new(pool.clone(), "/srv/images");
        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let state = crate::AppState {
            db: pool.clone(),
            hosts: repo.clone(),
            images,
            snapshots,
            allow_direct_image_paths: true,
        };

        let req = RegisterHostRequest {
            name: "agent-1".into(),
            addr: "http://127.0.0.1:9090".into(),
            capabilities: json!({"cpus": 4}),
        };

        let Json(response) = super::register(Extension(state), Json(req)).await.unwrap();
        let stored = repo.get(response.id).await.unwrap();
        assert_eq!(stored.name, "agent-1");
        assert_eq!(stored.addr, "http://127.0.0.1:9090");
        assert_eq!(stored.capabilities_json, json!({"cpus": 4}));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn heartbeat_updates_last_seen_and_capabilities(pool: sqlx::PgPool) {
        let repo = crate::features::hosts::repo::HostRepository::new(pool.clone());
        let images =
            crate::features::images::repo::ImageRepository::new(pool.clone(), "/srv/images");
        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let state = crate::AppState {
            db: pool.clone(),
            hosts: repo.clone(),
            images,
            snapshots,
            allow_direct_image_paths: true,
        };

        let req = RegisterHostRequest {
            name: "agent-2".into(),
            addr: "http://127.0.0.1:9191".into(),
            capabilities: json!({}),
        };

        let Json(register_resp) = super::register(Extension(state.clone()), Json(req))
            .await
            .unwrap();

        sqlx::query("UPDATE host SET last_seen_at = now() - interval '1 hour' WHERE id=$1")
            .bind(register_resp.id)
            .execute(repo.pool())
            .await
            .unwrap();

        let before = repo.get(register_resp.id).await.unwrap();

        let Json(response) = super::heartbeat(
            Extension(state),
            Path(HostPathParams {
                id: register_resp.id,
            }),
            Json(HostHeartbeatRequest {
                capabilities: Some(json!({"memory": 8192})),
            }),
        )
        .await
        .unwrap();

        assert_eq!(response, OkResponse::default());

        let after = repo.get(register_resp.id).await.unwrap();
        assert!(after.last_seen_at > before.last_seen_at);
        assert_eq!(after.capabilities_json, json!({"memory": 8192}));
    }
}
