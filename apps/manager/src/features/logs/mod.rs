use axum::{extract::Query, routing::get, Json, Router};
use serde::Deserialize;

#[derive(Deserialize)]
struct FileQuery {
    path: String,
}

pub fn router() -> Router {
    Router::new().route("/tail", get(tail_once))
}

/// Super simple file read (dev only). Frontend can poll.
async fn tail_once(Query(q): Query<FileQuery>) -> Json<serde_json::Value> {
    let txt = tokio::fs::read_to_string(q.path).await.unwrap_or_default();
    Json(serde_json::json!({"text": txt}))
}
