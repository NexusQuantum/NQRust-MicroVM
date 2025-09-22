use axum::{extract::Path, Json, Extension};
use uuid::Uuid;
use crate::AppState;
use nexus_types::CreateVmReq;


pub async fn create(Extension(st): Extension<AppState>, Json(req): Json<CreateVmReq>) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let id = Uuid::new_v4();
    super::service::create_and_start(&st, id, req).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"id": id})))
}


pub async fn list(Extension(st): Extension<AppState>) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let items = super::repo::list(&st.db).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"items": items})))
}


pub async fn get(Extension(st): Extension<AppState>, Path(id): Path<Uuid>) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let row = super::repo::get(&st.db, id).await.map_err(|_| axum::http::StatusCode::NOT_FOUND)?;
    Ok(Json(serde_json::json!({"item": row})))
}


pub async fn stop(Extension(st): Extension<AppState>, Path(id): Path<Uuid>) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    super::service::stop_only(&st, id).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"ok": true})))
}


pub async fn delete(Extension(st): Extension<AppState>, Path(id): Path<Uuid>) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    super::service::stop_and_delete(&st, id).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"ok": true})))
}