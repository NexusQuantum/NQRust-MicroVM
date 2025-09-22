use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::Response;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Request};
use hyperlocal::UnixConnector;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};

/// Forward HTTP over Unix-domain socket to Firecracker API without socat.
pub async fn forward(
    sock_path: &str,
    path: &str,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, (StatusCode, String)> {
    let client: Client<UnixConnector, Full<Bytes>> = Client::builder(TokioExecutor::new()).build(UnixConnector);
    let uri = hyperlocal::Uri::new(sock_path, path);
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(headers_mut) = req.headers_mut() {
        for (name, value) in headers.iter() {
            if name.as_str().eq_ignore_ascii_case("host") {
                continue;
            }
            headers_mut.insert(name, value.clone());
        }
    }
    let req = req.body(Full::new(body)).map_err(int)?;
    let res = client.request(req).await.map_err(int)?;
    let status = res.status();
    let body_bytes = res.into_body().collect().await.map_err(int)?.to_bytes();
    let resp = Response::builder()
        .status(status)
        .body(axum::body::Body::from(body_bytes))
        .map_err(int)?;
    Ok(resp)
}
fn int<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::BAD_GATEWAY, e.to_string())
}
