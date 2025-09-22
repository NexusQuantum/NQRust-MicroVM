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
