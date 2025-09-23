use crate::AppState;
use axum::{extract::Path, Extension, Json};
use nexus_types::CreateVmReq;
use uuid::Uuid;

pub async fn create(
    Extension(st): Extension<AppState>,
    Json(req): Json<CreateVmReq>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let id = Uuid::new_v4();
    super::service::create_and_start(&st, id, req)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"id": id})))
}

pub async fn list(
    Extension(st): Extension<AppState>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let items = super::repo::list(&st.db)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"items": items})))
}

pub async fn get(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let row = super::repo::get(&st.db, id)
        .await
        .map_err(|_| axum::http::StatusCode::NOT_FOUND)?;
    Ok(Json(serde_json::json!({"item": row})))
}

pub async fn stop(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    super::service::stop_only(&st, id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

pub async fn delete(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    super::service::stop_and_delete(&st, id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use anyhow::{anyhow, Result};
    use axum::Extension;
    use sqlx::postgres::PgPoolOptions;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn stop_handler_marks_vm_stopped() -> Result<()> {
        super::super::repo::reset_store();

        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://test:test@localhost:5432/test")
            .map_err(|err| anyhow!("failed to create lazy pool: {err}"))?;

        let mock_server = MockServer::start().await;
        let vm_id = Uuid::new_v4();
        let vm_row = super::super::repo::VmRow {
            id: vm_id,
            name: "test".into(),
            state: "running".into(),
            host_addr: mock_server.uri(),
            api_sock: format!("/srv/fc/vms/{vm_id}/sock/fc.sock"),
            tap: format!("tap-{vm_id}"),
            log_path: format!("/srv/fc/vms/{vm_id}/logs/firecracker.log"),
            http_port: 0,
            fc_unit: format!("fc-{vm_id}.scope"),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        super::super::repo::insert(&pool, &vm_row).await?;

        Mock::given(method("POST"))
            .and(path(format!("/agent/v1/vms/{}/stop", vm_id)))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let state = AppState {
            db: pool.clone(),
            agent_base: mock_server.uri(),
        };

        let _ = super::stop(Extension(state.clone()), axum::extract::Path(vm_id))
            .await
            .map_err(|status| anyhow!("handler failed with status {status}"))?;

        let row = super::super::repo::get(&state.db, vm_id).await?;
        assert_eq!(row.state, "stopped");

        mock_server.verify().await;

        Ok(())
    }
}
