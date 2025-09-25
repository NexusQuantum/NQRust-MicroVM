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
pub struct TemplateSpec {
    pub vcpu: u8,
    pub mem_mib: u32,
    pub kernel_path: String,
    pub rootfs_path: String,
}

impl TemplateSpec {
    pub fn into_vm_req(self, name: String) -> CreateVmReq {
        CreateVmReq {
            name,
            vcpu: self.vcpu,
            mem_mib: self.mem_mib,
            kernel_path: self.kernel_path,
            rootfs_path: self.rootfs_path,
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
