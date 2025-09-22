use axum::{routing::any, extract::{Path, Query}, http::{Method, HeaderMap, StatusCode}, Router};
use bytes::Bytes;
use crate::core::uds_proxy;
use crate::AppState;
use axum::Extension;


#[derive(serde::Deserialize)]
struct ProxyQuery { sock: String }


pub fn router() -> Router { Router::new().route("/:id/proxy/*path", any(proxy)) }


async fn proxy(Extension(_st): Extension<AppState>, Path((_id, path)): Path<(String, String)>, Query(q): Query<ProxyQuery>, method: Method, headers: HeaderMap, body: Bytes) -> Result<axum::response::Response, (StatusCode, String)> {
    uds_proxy::forward(&q.sock, &format!("/{}", path), method, headers, body).await
}