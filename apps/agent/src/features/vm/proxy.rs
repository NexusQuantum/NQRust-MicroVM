use std::path::Component;

use axum::{
    extract::{Path, Query},
    http::{HeaderMap, Method, StatusCode},
    routing::any,
    Extension, Router,
};
use bytes::Bytes;

use crate::{core::uds_proxy, AppState};

#[derive(serde::Deserialize)]
struct ProxyQuery {
    sock: String,
}

pub fn router() -> Router {
    Router::new().route("/:id/proxy/*path", any(proxy))
}

async fn proxy(
    Extension(st): Extension<AppState>,
    Path((id, path)): Path<(String, String)>,
    Query(q): Query<ProxyQuery>,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Result<axum::response::Response, (StatusCode, String)> {
    let sock = resolve_socket_path(&st, &id, &q.sock).await?;
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.is_empty()
        || segments.iter().any(|seg| *seg == "." || *seg == "..")
        || !is_allowed_endpoint(&method, &segments)
    {
        return Err((StatusCode::FORBIDDEN, "endpoint not allowed".into()));
    }
    let forward_path = format!("/{}", segments.join("/"));
    uds_proxy::forward(&sock, &forward_path, method, headers, body).await
}

async fn resolve_socket_path(
    st: &AppState,
    id: &str,
    requested: &str,
) -> Result<String, (StatusCode, String)> {
    let canonical_sock = tokio::fs::canonicalize(requested)
        .await
        .map_err(|_| (StatusCode::FORBIDDEN, "invalid socket".into()))?;

    let run_dir = tokio::fs::canonicalize(&st.run_dir)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "run_dir missing".into()))?;

    if !canonical_sock.starts_with(&run_dir) {
        return Err((StatusCode::FORBIDDEN, "socket outside run_dir".into()));
    }

    let id_component = std::ffi::OsStr::new(id);
    if !canonical_sock
        .components()
        .any(|c| matches!(c, Component::Normal(name) if name == id_component))
    {
        return Err((StatusCode::FORBIDDEN, "socket does not match vm".into()));
    }

    canonical_sock
        .to_str()
        .map(|s| s.to_owned())
        .ok_or_else(|| (StatusCode::FORBIDDEN, "socket path encoding".into()))
}

fn is_allowed_endpoint(method: &Method, segments: &[&str]) -> bool {
    match (method, segments) {
        (&Method::PUT, ["machine-config"]) => true,
        (&Method::PUT, ["boot-source"]) => true,
        (&Method::PUT, ["logger"]) => true,
        (&Method::PUT, ["metrics"]) => true,
        (&Method::PUT, ["actions"]) => true,
        (&Method::PUT, ["drives", _]) => true,
        (&Method::PUT, ["network-interfaces", _]) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn allows_valid_socket() {
        let tmp = tempfile::tempdir().unwrap();
        let run_dir = tmp.path().join("fc");
        let id = "vm-123";
        let sock_path = run_dir.join("vms").join(id).join("sock");
        std::fs::create_dir_all(&sock_path).unwrap();
        let sock_file = sock_path.join("fc.sock");
        std::fs::File::create(&sock_file).unwrap();

        let st = AppState {
            run_dir: run_dir.to_string_lossy().to_string(),
            bridge: "fcbr0".into(),
        };

        let resolved = resolve_socket_path(&st, id, sock_file.to_str().unwrap())
            .await
            .expect("socket should be valid");

        let expected = std::fs::canonicalize(sock_file)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(resolved, expected);
    }

    #[tokio::test]
    async fn blocks_traversal_outside_run_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let run_dir = tmp.path().join("fc");
        std::fs::create_dir_all(&run_dir).unwrap();
        let outside = tmp.path().join("outside.sock");
        std::fs::File::create(&outside).unwrap();

        let st = AppState {
            run_dir: run_dir.to_string_lossy().to_string(),
            bridge: "fcbr0".into(),
        };

        let nested = run_dir.join("vms").join("vm-abc").join("sock");
        std::fs::create_dir_all(&nested).unwrap();
        let traversal = nested.join("../../../../outside.sock");
        let traversal_str = traversal.to_string_lossy().to_string();

        let err = resolve_socket_path(&st, "vm-abc", &traversal_str)
            .await
            .expect_err("traversal must be blocked");

        assert_eq!(err.0, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn blocks_socket_for_other_vm() {
        let tmp = tempfile::tempdir().unwrap();
        let run_dir = tmp.path().join("fc");
        let actual_vm_id = "vm-real";
        let socket_dir = run_dir.join("vms").join(actual_vm_id).join("sock");
        std::fs::create_dir_all(&socket_dir).unwrap();
        let sock = socket_dir.join("fc.sock");
        std::fs::File::create(&sock).unwrap();

        let st = AppState {
            run_dir: run_dir.to_string_lossy().to_string(),
            bridge: "fcbr0".into(),
        };

        let err = resolve_socket_path(&st, "vm-other", sock.to_str().unwrap())
            .await
            .expect_err("should reject sockets for other VMs");

        assert_eq!(err.0, StatusCode::FORBIDDEN);
    }
}
