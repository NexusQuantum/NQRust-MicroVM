use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Generate a stable device fingerprint based on hostname, OS, arch, and CPU model.
fn generate_device_id() -> String {
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_default();
    let platform = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let cpu_model = read_cpu_model().unwrap_or_default();

    let info = format!("{}|{}|{}|{}", hostname, platform, arch, cpu_model);
    let hash = Sha256::digest(info.as_bytes());
    hex::encode(&hash[..16]) // 32-char hex string
}

fn read_cpu_model() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
            for line in cpuinfo.lines() {
                if line.starts_with("model name") {
                    if let Some(val) = line.split(':').nth(1) {
                        return Some(val.trim().to_string());
                    }
                }
            }
        }
        None
    }

    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Get or create a persistent device ID stored in `<persist_dir>/.device-id`.
pub fn get_or_create_device_id(persist_dir: &str) -> String {
    let id_file = Path::new(persist_dir).join(".device-id");

    if let Ok(existing) = fs::read_to_string(&id_file) {
        let trimmed = existing.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }

    let device_id = generate_device_id();
    let _ = fs::create_dir_all(persist_dir);
    let _ = fs::write(&id_file, &device_id);
    device_id
}
