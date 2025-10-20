use axum::{
    routing::{get, post},
    Router,
};

pub mod repo; // db
pub mod routes; // handlers
pub mod service; // orchestration
pub mod shell; // shell session helpers

pub fn router() -> Router {
    Router::new()
        .route("/", post(routes::create).get(routes::list))
        .route("/:id", get(routes::get).delete(routes::delete))
        .route("/:id/start", post(routes::start))
        .route("/:id/stop", post(routes::stop))
        .route("/:id/pause", post(routes::pause))
        .route("/:id/resume", post(routes::resume))
        .route("/:id/flush-metrics", post(routes::flush_metrics))
        .route("/:id/ctrl-alt-del", post(routes::ctrl_alt_del))
        .route(
            "/:id/drives",
            get(routes::list_drives).post(routes::create_drive),
        )
        .route(
            "/:id/drives/:drive_id",
            get(routes::get_drive)
                .patch(routes::update_drive)
                .delete(routes::delete_drive),
        )
        .route("/:id/nics", get(routes::list_nics).post(routes::create_nic))
        .route(
            "/:id/nics/:nic_id",
            get(routes::get_nic)
                .patch(routes::update_nic)
                .delete(routes::delete_nic),
        )
        .route("/:id/shell", get(routes::get_shell_credentials))
        .route("/:id/shell/ws", get(routes::shell_websocket))
        .route("/:id/metrics/ws", get(routes::metrics_websocket))
        .route(
            "/:id/machine-config",
            axum::routing::patch(routes::patch_machine_config),
        )
        .route(
            "/:id/cpu-config",
            axum::routing::put(routes::put_cpu_config),
        )
        .route("/:id/vsock", axum::routing::put(routes::put_vsock))
        .route("/:id/mmds", axum::routing::put(routes::put_mmds))
        .route(
            "/:id/mmds/config",
            axum::routing::put(routes::put_mmds_config),
        )
        .route("/:id/entropy", axum::routing::put(routes::put_entropy))
        .route("/:id/serial", axum::routing::put(routes::put_serial))
        .route("/:id/logger", axum::routing::put(routes::put_logger))
        .route(
            "/:id/balloon",
            axum::routing::put(routes::put_balloon).patch(routes::patch_balloon),
        )
        .route(
            "/:id/balloon/statistics",
            axum::routing::patch(routes::patch_balloon_statistics),
        )
}
