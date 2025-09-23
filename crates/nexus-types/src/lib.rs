use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVmReq {
    pub name: String,
    pub vcpu: u8,
    pub mem_mib: u32,
    pub kernel_path: String,
    pub rootfs_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmSummary {
    pub id: uuid::Uuid,
    pub name: String,
    pub state: String,
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
