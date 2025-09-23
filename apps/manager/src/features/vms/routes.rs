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
    use axum::{extract::Path, Extension};
    use serde_json::json;

    #[sqlx::test(migrations = "./migrations")]
    async fn delete_route_removes_vm(pool: sqlx::PgPool) {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let row = super::super::repo::VmRow {
            id,
            name: "test-vm".into(),
            state: "running".into(),
            host_addr: "http://127.0.0.1:1".into(),
            api_sock: "/tmp/test.sock".into(),
            tap: "tap-test".into(),
            log_path: "/tmp/log".into(),
            http_port: 0,
            fc_unit: "fc-test.scope".into(),
            created_at: now,
            updated_at: now,
        };
        super::super::repo::insert(&pool, &row).await.unwrap();

        let state = crate::AppState {
            db: pool.clone(),
            agent_base: row.host_addr.clone(),
        };

        let Json(body) = super::delete(Extension(state), Path(id)).await.unwrap();
        assert_eq!(body, json!({"ok": true}));

        let fetched = super::super::repo::get(&pool, id).await;
        assert!(matches!(fetched, Err(sqlx::Error::RowNotFound)));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn delete_route_unknown_id_returns_ok(pool: sqlx::PgPool) {
        let state = crate::AppState {
            db: pool,
            agent_base: "http://127.0.0.1:1".into(),
        };
        let Json(body) = super::delete(Extension(state), Path(Uuid::new_v4()))
            .await
            .unwrap();
        assert_eq!(body, json!({"ok": true}));
    }
}
