use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVmReq {
    pub name: String,
    pub vcpu: u8,
    pub mem_mib: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernel_image_id: Option<uuid::Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rootfs_image_id: Option<uuid::Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernel_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rootfs_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_snapshot_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSpec {
    pub vcpu: u8,
    pub mem_mib: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernel_image_id: Option<uuid::Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rootfs_image_id: Option<uuid::Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernel_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rootfs_path: Option<String>,
}

impl TemplateSpec {
    pub fn into_vm_req(self, name: String) -> CreateVmReq {
        CreateVmReq {
            name,
            vcpu: self.vcpu,
            mem_mib: self.mem_mib,
            kernel_image_id: self.kernel_image_id,
            rootfs_image_id: self.rootfs_image_id,
            kernel_path: self.kernel_path,
            rootfs_path: self.rootfs_path,
            source_snapshot_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTemplateReq {
    pub name: String,
    pub spec: TemplateSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: uuid::Uuid,
    pub name: String,
    pub spec: TemplateSpec,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTemplateResp {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTemplatesResp {
    pub items: Vec<Template>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTemplateResp {
    pub item: Template,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstantiateTemplateReq {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstantiateTemplateResp {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmSummary {
    pub id: uuid::Uuid,
    pub name: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: uuid::Uuid,
    pub vm_id: uuid::Uuid,
    pub snapshot_path: String,
    pub mem_path: String,
    pub size_bytes: i64,
    pub state: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateSnapshotRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSnapshotResponse {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSnapshotsResponse {
    pub items: Vec<Snapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSnapshotResponse {
    pub item: Snapshot,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstantiateSnapshotReq {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstantiateSnapshotResp {
    pub id: uuid::Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub id: uuid::Uuid,
    pub kind: String,
    pub name: String,
    pub host_path: String,
    pub sha256: String,
    pub size: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateImageReq {
    pub kind: String,
    pub name: String,
    pub host_path: String,
    pub sha256: String,
    pub size: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateImageResp {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListImagesResp {
    pub items: Vec<Image>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetImageResp {
    pub item: Image,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterHostRequest {
    pub name: String,
    pub addr: String,
    #[serde(default)]
    pub capabilities: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterHostResponse {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostHeartbeatRequest {
    #[serde(default)]
    pub capabilities: Option<serde_json::Value>,
}
