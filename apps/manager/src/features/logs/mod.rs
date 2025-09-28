use axum::{extract::Query, routing::get, Json, Router};
use nexus_types::TailLogResponse;
use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TailLogQuery {
    path: String,
}

pub fn router() -> Router {
    Router::new().route("/tail", get(tail_once))
}

/// Super simple file read (dev only). Frontend can poll.
#[utoipa::path(
    get,
    path = "/v1/logs/tail",
    params(TailLogQuery),
    responses((status = 200, description = "Log tailed", body = TailLogResponse)),
    tag = "Logs"
)]
pub async fn tail_once(Query(q): Query<TailLogQuery>) -> Json<TailLogResponse> {
    let txt = tokio::fs::read_to_string(q.path).await.unwrap_or_default();
    Json(TailLogResponse { text: txt })
}
