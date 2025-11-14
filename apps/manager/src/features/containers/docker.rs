use anyhow::{anyhow, Context, Result};
use nexus_types::CreateContainerReq;
use serde::Deserialize;

/// Docker client that communicates with Docker daemon inside a VM via HTTP
pub struct DockerClient {
    base_url: String,
    client: reqwest::Client,
}

impl DockerClient {
    /// Create a new Docker client for a specific guest VM
    pub fn new(guest_ip: &str) -> Result<Self> {
        let base_url = format!("http://{}:2375", guest_ip);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self { base_url, client })
    }

    /// Create a container from the specification
    pub async fn create_container(&self, req: &CreateContainerReq) -> Result<String> {
        let url = format!("{}/containers/create?name={}", self.base_url, req.name);

        // Build Docker container config
        let env: Vec<String> = req
            .env_vars
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        let mut exposed_ports = serde_json::Map::new();
        let mut port_bindings = serde_json::Map::new();

        for mapping in &req.port_mappings {
            let container_port = format!("{}/{}", mapping.container, mapping.protocol);
            exposed_ports.insert(container_port.clone(), serde_json::json!({}));

            let host_binding = serde_json::json!([{
                "HostIp": "",
                "HostPort": mapping.host.to_string()
            }]);
            port_bindings.insert(container_port, host_binding);
        }

        let mut binds: Vec<String> = vec![];
        for volume in &req.volumes {
            let bind = if volume.read_only {
                format!("{}:{}:ro", volume.host, volume.container)
            } else {
                format!("{}:{}", volume.host, volume.container)
            };
            binds.push(bind);
        }

        let host_config = serde_json::json!({
            "Binds": binds,
            "PortBindings": port_bindings,
            "Memory": req.memory_limit_mb.map(|m| m as i64 * 1024 * 1024),
            "NanoCpus": req.cpu_limit.map(|c| (c * 1_000_000_000.0) as i64),
            "RestartPolicy": {
                "Name": req.restart_policy
            }
        });

        let config = serde_json::json!({
            "Image": req.image,
            "Cmd": if !req.args.is_empty() { Some(&req.args) } else { None },
            "Entrypoint": req.command.as_ref().map(|c| vec![c]),
            "Env": env,
            "ExposedPorts": exposed_ports,
            "HostConfig": host_config,
        });

        tracing::info!(
            image = %req.image,
            name = %req.name,
            "Creating container via Docker API"
        );

        let resp = self
            .client
            .post(&url)
            .json(&config)
            .send()
            .await
            .context("Failed to send create container request")?;

        if !resp.status().is_success() {
            let error_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to create container: {}", error_text);
        }

        let result: CreateContainerResponse = resp.json().await?;
        Ok(result.id)
    }

    /// Start a container
    pub async fn start_container(&self, container_id: &str) -> Result<()> {
        let url = format!("{}/containers/{}/start", self.base_url, container_id);

        tracing::info!(container_id = %container_id, "Starting container");

        let resp = self.client.post(&url).send().await?;

        if !resp.status().is_success() && resp.status().as_u16() != 304 {
            // 304 = already started
            let error_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to start container: {}", error_text);
        }

        Ok(())
    }

    /// Stop a container
    pub async fn stop_container(&self, container_id: &str, timeout: Option<i64>) -> Result<()> {
        let timeout_param = timeout.unwrap_or(10);
        let url = format!(
            "{}/containers/{}/stop?t={}",
            self.base_url, container_id, timeout_param
        );

        tracing::info!(container_id = %container_id, timeout = timeout_param, "Stopping container");

        let resp = self.client.post(&url).send().await?;

        if !resp.status().is_success() && resp.status().as_u16() != 304 {
            // 304 = already stopped
            let error_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to stop container: {}", error_text);
        }

        Ok(())
    }

    /// Restart a container
    pub async fn restart_container(&self, container_id: &str, timeout: Option<i64>) -> Result<()> {
        let timeout_param = timeout.unwrap_or(10);
        let url = format!(
            "{}/containers/{}/restart?t={}",
            self.base_url, container_id, timeout_param
        );

        tracing::info!(container_id = %container_id, "Restarting container");

        let resp = self.client.post(&url).send().await?;

        if !resp.status().is_success() {
            let error_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to restart container: {}", error_text);
        }

        Ok(())
    }

    /// Pause a container
    pub async fn pause_container(&self, container_id: &str) -> Result<()> {
        let url = format!("{}/containers/{}/pause", self.base_url, container_id);

        tracing::info!(container_id = %container_id, "Pausing container");

        let resp = self.client.post(&url).send().await?;

        if !resp.status().is_success() {
            let error_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to pause container: {}", error_text);
        }

        Ok(())
    }

    /// Unpause a container
    pub async fn unpause_container(&self, container_id: &str) -> Result<()> {
        let url = format!("{}/containers/{}/unpause", self.base_url, container_id);

        tracing::info!(container_id = %container_id, "Unpausing container");

        let resp = self.client.post(&url).send().await?;

        if !resp.status().is_success() {
            let error_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to unpause container: {}", error_text);
        }

        Ok(())
    }

    /// Remove a container
    #[allow(dead_code)]
    pub async fn remove_container(&self, container_id: &str, force: bool) -> Result<()> {
        let url = format!(
            "{}/containers/{}?force={}",
            self.base_url, container_id, force
        );

        tracing::info!(container_id = %container_id, force = force, "Removing container");

        let resp = self.client.delete(&url).send().await?;

        if !resp.status().is_success() {
            let error_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to remove container: {}", error_text);
        }

        Ok(())
    }

    /// Get container stats
    pub async fn get_stats(&self, container_id: &str) -> Result<DockerStats> {
        let url = format!(
            "{}/containers/{}/stats?stream=false",
            self.base_url, container_id
        );

        tracing::debug!(container_id = %container_id, "Getting container stats");

        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            let error_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to get stats: {}", error_text);
        }

        let stats: DockerStatsResponse = resp.json().await?;

        // Calculate CPU percentage
        let cpu_percent = calculate_cpu_percent(&stats);

        // Extract memory stats
        let memory_used_mb = (stats.memory_stats.usage.unwrap_or(0) / 1024 / 1024) as i64;
        let memory_limit_mb = (stats.memory_stats.limit.unwrap_or(0) / 1024 / 1024) as i64;

        // Extract network stats
        let (network_rx_bytes, network_tx_bytes) = extract_network_stats(&stats);

        // Extract block I/O stats
        let (block_read_bytes, block_write_bytes) = extract_block_io_stats(&stats);

        Ok(DockerStats {
            cpu_percent: Some(cpu_percent),
            memory_used_mb: Some(memory_used_mb),
            memory_limit_mb: Some(memory_limit_mb),
            network_rx_bytes: Some(network_rx_bytes),
            network_tx_bytes: Some(network_tx_bytes),
            block_read_bytes: Some(block_read_bytes),
            block_write_bytes: Some(block_write_bytes),
            pids: stats.pids_stats.current.map(|p| p as i32),
        })
    }

    /// Get container logs
    #[allow(dead_code)]
    pub async fn get_logs(
        &self,
        container_id: &str,
        tail: Option<i64>,
        since: Option<i64>,
    ) -> Result<Vec<LogEntry>> {
        let mut url = format!(
            "{}/containers/{}/logs?stdout=true&stderr=true&timestamps=true",
            self.base_url, container_id
        );

        if let Some(tail_lines) = tail {
            url.push_str(&format!("&tail={}", tail_lines));
        }

        if let Some(since_timestamp) = since {
            url.push_str(&format!("&since={}", since_timestamp));
        }

        tracing::debug!(container_id = %container_id, "Getting container logs");

        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            let error_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to get logs: {}", error_text);
        }

        let logs_text = resp.text().await?;

        // Parse Docker log format (stream header + data)
        // For simplicity, just split by newlines
        let entries: Vec<LogEntry> = logs_text
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| {
                // Docker log format: [timestamp] message
                // We'll do basic parsing
                LogEntry {
                    timestamp: chrono::Utc::now(),
                    stream: "stdout".to_string(),
                    message: line.to_string(),
                }
            })
            .collect();

        Ok(entries)
    }

    /// Execute a command in a container
    pub async fn exec_command(
        &self,
        container_id: &str,
        command: Vec<String>,
        _attach_stdout: bool,
        _attach_stderr: bool,
    ) -> Result<ExecResult> {
        // Create exec instance
        let create_url = format!("{}/containers/{}/exec", self.base_url, container_id);

        let create_config = serde_json::json!({
            "Cmd": command,
            "AttachStdout": true,
            "AttachStderr": true,
        });

        let create_resp = self
            .client
            .post(&create_url)
            .json(&create_config)
            .send()
            .await?;

        if !create_resp.status().is_success() {
            let error_text = create_resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to create exec: {}", error_text);
        }

        let create_result: serde_json::Value = create_resp.json().await?;
        let exec_id = create_result["Id"]
            .as_str()
            .ok_or_else(|| anyhow!("No exec ID returned"))?;

        // Start exec instance
        let start_url = format!("{}/exec/{}/start", self.base_url, exec_id);

        let start_config = serde_json::json!({
            "Detach": false,
        });

        let start_resp = self
            .client
            .post(&start_url)
            .json(&start_config)
            .send()
            .await?;

        let output = if start_resp.status().is_success() {
            start_resp.text().await.ok()
        } else {
            None
        };

        Ok(ExecResult {
            exec_id: exec_id.to_string(),
            output,
            exit_code: Some(0), // TODO: Get actual exit code from inspect
        })
    }

    /// Pull an image from a registry with optional authentication
    pub async fn pull_image(
        &self,
        image: &str,
        registry_auth: Option<&nexus_types::RegistryAuth>,
    ) -> Result<()> {
        let url = format!("{}/images/create?fromImage={}", self.base_url, image);

        tracing::info!(image = %image, has_auth = registry_auth.is_some(), "Pulling image");

        let mut request = self.client.post(&url);

        // Add authentication header if provided
        if let Some(auth) = registry_auth {
            // Docker expects X-Registry-Auth header with base64-encoded JSON
            let auth_config = serde_json::json!({
                "username": auth.username,
                "password": auth.password,
                "serveraddress": auth.server_address.as_deref().unwrap_or("https://index.docker.io/v1/"),
            });

            let auth_json = serde_json::to_string(&auth_config)?;
            let auth_base64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                auth_json.as_bytes(),
            );

            request = request.header("X-Registry-Auth", auth_base64);
            tracing::debug!("Added registry authentication header");
        }

        let resp = request.send().await?;

        if !resp.status().is_success() {
            let error_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to pull image: {}", error_text);
        }

        // Docker streams the pull progress, we just wait for completion
        let _ = resp.text().await?;

        Ok(())
    }

    /// Check if an image exists locally
    pub async fn image_exists(&self, image: &str) -> Result<bool> {
        let url = format!("{}/images/{}/json", self.base_url, image);

        tracing::debug!(image = %image, "Checking if image exists");

        let resp = self.client.get(&url).send().await?;

        Ok(resp.status().is_success())
    }
}

// Response types from Docker API

#[derive(Debug, Deserialize)]
struct CreateContainerResponse {
    #[serde(rename = "Id")]
    id: String,
}

#[derive(Debug, Deserialize)]
struct DockerStatsResponse {
    cpu_stats: CpuStats,
    precpu_stats: CpuStats,
    memory_stats: MemoryStats,
    #[serde(default)]
    networks: std::collections::HashMap<String, NetworkStats>,
    #[serde(default)]
    blkio_stats: BlkioStats,
    pids_stats: PidsStats,
}

#[derive(Debug, Deserialize, Default)]
struct CpuStats {
    cpu_usage: CpuUsage,
    system_cpu_usage: Option<u64>,
}

#[derive(Debug, Deserialize, Default)]
struct CpuUsage {
    total_usage: u64,
}

#[derive(Debug, Deserialize)]
struct MemoryStats {
    usage: Option<u64>,
    limit: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct NetworkStats {
    rx_bytes: u64,
    tx_bytes: u64,
}

#[derive(Debug, Deserialize, Default)]
struct BlkioStats {
    #[serde(default)]
    io_service_bytes_recursive: Vec<BlkioStatEntry>,
}

#[derive(Debug, Deserialize)]
struct BlkioStatEntry {
    op: String,
    value: u64,
}

#[derive(Debug, Deserialize)]
struct PidsStats {
    current: Option<u64>,
}

// Exported types

pub struct DockerStats {
    pub cpu_percent: Option<f32>,
    pub memory_used_mb: Option<i64>,
    pub memory_limit_mb: Option<i64>,
    pub network_rx_bytes: Option<i64>,
    pub network_tx_bytes: Option<i64>,
    pub block_read_bytes: Option<i64>,
    pub block_write_bytes: Option<i64>,
    pub pids: Option<i32>,
}

#[allow(dead_code)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub stream: String,
    pub message: String,
}

pub struct ExecResult {
    pub exec_id: String,
    pub output: Option<String>,
    pub exit_code: Option<i32>,
}

// Helper functions

fn calculate_cpu_percent(stats: &DockerStatsResponse) -> f32 {
    let cpu_delta = stats.cpu_stats.cpu_usage.total_usage as f64
        - stats.precpu_stats.cpu_usage.total_usage as f64;

    let system_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0) as f64
        - stats.precpu_stats.system_cpu_usage.unwrap_or(0) as f64;

    if system_delta > 0.0 && cpu_delta > 0.0 {
        (cpu_delta / system_delta * 100.0) as f32
    } else {
        0.0
    }
}

fn extract_network_stats(stats: &DockerStatsResponse) -> (i64, i64) {
    let mut total_rx = 0i64;
    let mut total_tx = 0i64;

    for net_stats in stats.networks.values() {
        total_rx += net_stats.rx_bytes as i64;
        total_tx += net_stats.tx_bytes as i64;
    }

    (total_rx, total_tx)
}

fn extract_block_io_stats(stats: &DockerStatsResponse) -> (i64, i64) {
    let mut total_read = 0i64;
    let mut total_write = 0i64;

    for entry in &stats.blkio_stats.io_service_bytes_recursive {
        match entry.op.as_str() {
            "Read" => total_read += entry.value as i64,
            "Write" => total_write += entry.value as i64,
            _ => {}
        }
    }

    (total_read, total_write)
}
