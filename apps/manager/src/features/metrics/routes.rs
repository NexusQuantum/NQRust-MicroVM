use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use axum::Json;
use nexus_types::{ContainerMetric, HostMetric, MetricsQueryParams, VmMetric};
use uuid::Uuid;

use crate::features::metrics::repo;
use crate::AppState;

const DEFAULT_LIMIT: i64 = 360; // 1 hour at 10s intervals

pub async fn get_host_metrics(
    Extension(state): Extension<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<MetricsQueryParams>,
) -> Result<Json<Vec<HostMetric>>, StatusCode> {
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).min(2160); // max 6h
    repo::query_host_metrics(&state.db, id, params.from, params.to, limit)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn get_vm_metrics(
    Extension(state): Extension<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<MetricsQueryParams>,
) -> Result<Json<Vec<VmMetric>>, StatusCode> {
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).min(2160);
    repo::query_vm_metrics(&state.db, id, params.from, params.to, limit)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn get_container_metrics(
    Extension(state): Extension<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<MetricsQueryParams>,
) -> Result<Json<Vec<ContainerMetric>>, StatusCode> {
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).min(2160);
    repo::query_container_metrics(&state.db, id, params.from, params.to, limit)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
