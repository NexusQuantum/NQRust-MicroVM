use axum::Router;
use utoipa::openapi::OpenApi as OpenApiDoc;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::features::hosts::routes::register,
        crate::features::hosts::routes::heartbeat,
        crate::features::templates::routes::create,
        crate::features::templates::routes::list,
        crate::features::templates::routes::get,
        crate::features::templates::routes::instantiate,
        crate::features::vms::routes::create,
        crate::features::vms::routes::list,
        crate::features::vms::routes::get,
        crate::features::vms::routes::stop,
        crate::features::vms::routes::delete,
        crate::features::vms::routes::pause,
        crate::features::vms::routes::resume,
        crate::features::vms::routes::flush_metrics,
        crate::features::vms::routes::ctrl_alt_del,
        crate::features::vms::routes::list_drives,
        crate::features::vms::routes::create_drive,
        crate::features::vms::routes::get_drive,
        crate::features::vms::routes::update_drive,
        crate::features::vms::routes::delete_drive,
        crate::features::vms::routes::list_nics,
        crate::features::vms::routes::create_nic,
        crate::features::vms::routes::get_nic,
        crate::features::vms::routes::update_nic,
        crate::features::vms::routes::delete_nic,
        crate::features::images::routes::create,
        crate::features::images::routes::list,
        crate::features::images::routes::get,
        crate::features::images::routes::delete,
        crate::features::snapshots::routes::create,
        crate::features::snapshots::routes::list_for_vm,
        crate::features::snapshots::routes::get,
        crate::features::snapshots::routes::instantiate,
        crate::features::logs::tail_once,
        crate::features::vms::routes::put_entropy,
        crate::features::vms::routes::put_serial,
        crate::features::vms::routes::put_logger,
        crate::features::vms::routes::put_balloon,
        crate::features::vms::routes::patch_balloon,
        crate::features::vms::routes::patch_balloon_statistics,
    ),
    components(
        schemas(
            nexus_types::RegisterHostRequest,
            nexus_types::RegisterHostResponse,
            nexus_types::HostHeartbeatRequest,
            nexus_types::OkResponse,
            nexus_types::CreateTemplateReq,
            nexus_types::CreateTemplateResp,
            nexus_types::ListTemplatesResp,
            nexus_types::GetTemplateResp,
            nexus_types::InstantiateTemplateReq,
            nexus_types::InstantiateTemplateResp,
            nexus_types::TemplateSpec,
            nexus_types::CreateVmReq,
            nexus_types::CreateVmResponse,
            nexus_types::ListVmsResponse,
            nexus_types::GetVmResponse,
            nexus_types::Vm,
            nexus_types::CreateImageReq,
            nexus_types::CreateImageResp,
            nexus_types::ListImagesResp,
            nexus_types::GetImageResp,
            nexus_types::Image,
            nexus_types::CreateSnapshotRequest,
            nexus_types::CreateSnapshotResponse,
            nexus_types::ListSnapshotsResponse,
            nexus_types::GetSnapshotResponse,
            nexus_types::Snapshot,
            nexus_types::InstantiateSnapshotReq,
            nexus_types::InstantiateSnapshotResp,
            nexus_types::TailLogResponse,
            nexus_types::VmDrive,
            nexus_types::CreateDriveReq,
            nexus_types::UpdateDriveReq,
            nexus_types::ListDrivesResponse,
            nexus_types::VmNic,
            nexus_types::CreateNicReq,
            nexus_types::UpdateNicReq,
            nexus_types::ListNicsResponse,
            nexus_types::ListVmsResponse,
            nexus_types::LoggerUpdateReq,
            nexus_types::BalloonConfig,
            nexus_types::BalloonStatsConfig,
        )
    ),
    tags(
        (name = "Hosts", description = "Host lifecycle operations."),
        (name = "Templates", description = "Template management APIs."),
        (name = "VMs", description = "Virtual machine lifecycle APIs."),
        (name = "Images", description = "Image registry APIs."),
        (name = "Snapshots", description = "Snapshot management APIs."),
        (name = "Logs", description = "Development log utilities."),
        (name = "VM devices", description = "Block and network device management."),
    )
)]
pub struct ApiDoc;

pub fn router(openapi: OpenApiDoc) -> Router {
    Router::new().merge(SwaggerUi::new("/docs").url("/docs/openapi.json", openapi))
}

pub async fn write_openapi_yaml(openapi: &OpenApiDoc) -> anyhow::Result<()> {
    let yaml = serde_yaml::to_string(openapi)?;
    tokio::fs::create_dir_all("openapi/manager").await?;
    tokio::fs::write("openapi/manager/openapi.yaml", yaml).await?;
    Ok(())
}
