use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::process::Command;
use bollard::Docker;
use bollard::image::CreateImageOptions;
use futures::StreamExt;

/// Docker Hub API client for searching and downloading images
#[derive(Clone)]
pub struct DockerHubClient {
    image_root: PathBuf,
    auth_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DockerHubSearchResult {
    #[serde(default)]
    results: Vec<DockerHubRepo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DockerHubRepo {
    // Docker Hub API uses repo_name, but we map it to name
    #[serde(rename = "repo_name")]
    name: String,
    // Docker Hub API uses short_description
    #[serde(rename = "short_description", default)]
    description: Option<String>,
    #[serde(default)]
    star_count: i32,
    #[serde(default)]
    is_official: bool,
    #[serde(default)]
    is_automated: bool,
    #[serde(default)]
    pull_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct DockerHubTagsResult {
    #[serde(default)]
    results: Vec<DockerHubTag>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DockerHubTag {
    name: String,
    #[serde(default)]
    last_updated: Option<String>,
    #[serde(default)]
    digest: Option<String>,
    #[serde(default)]
    full_size: Option<i64>,
}

impl DockerHubClient {
    pub fn new(image_root: PathBuf) -> Self {
        // Get Docker Hub token from environment if available (optional)
        let auth_token = std::env::var("DOCKER_HUB_TOKEN")
            .ok()
            .filter(|t| !t.is_empty());
        
        Self { 
            image_root,
            auth_token,
        }
    }

    /// Search Docker Hub for images
    pub async fn search(&self, query: &str, limit: Option<i32>) -> Result<Vec<nexus_types::DockerHubImage>> {
        let limit = limit.unwrap_or(25).min(100);

        let url = format!(
            "https://hub.docker.com/v2/search/repositories/?query={}&page_size={}",
            urlencoding::encode(query),
            limit
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        let mut request = client.get(&url).header("Search-Version", "v3");
        
        // Add authentication if token is available (improves rate limits)
        if let Some(token) = &self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request
            .send()
            .await
            .context("Failed to search Docker Hub")?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("Failed to read Docker Hub search response")?;

        if !status.is_success() {
            anyhow::bail!(
                "Docker Hub search returned error status {}: {}",
                status,
                response_text.chars().take(500).collect::<String>()
            );
        }

        let search_result: DockerHubSearchResult = serde_json::from_str(&response_text)
            .with_context(|| {
                format!(
                    "Failed to parse Docker Hub search response. Status: {}, Response (first 500 chars): {}",
                    status,
                    response_text.chars().take(500).collect::<String>()
                )
            })?;

        Ok(search_result
            .results
            .into_iter()
            .map(|repo| nexus_types::DockerHubImage {
                name: repo.name,
                description: repo.description,
                star_count: repo.star_count,
                is_official: repo.is_official,
                is_automated: repo.is_automated,
                pull_count: repo.pull_count,
            })
            .collect())
    }

    /// Get tags for a Docker image
    pub async fn get_tags(&self, image_name: &str) -> Result<Vec<nexus_types::DockerImageTag>> {
        // Normalize image name (add library/ prefix for official images)
        let normalized_name = if image_name.contains('/') {
            image_name.to_string()
        } else {
            format!("library/{}", image_name)
        };

        let url = format!(
            "https://hub.docker.com/v2/repositories/{}/tags/?page_size=100",
            normalized_name
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        let mut request = client.get(&url);
        
        // Add authentication if token is available (improves rate limits)
        if let Some(token) = &self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request
            .send()
            .await
            .context("Failed to get Docker image tags")?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("Failed to read Docker Hub tags response")?;

        if !status.is_success() {
            anyhow::bail!(
                "Docker Hub tags returned error status {}: {}",
                status,
                response_text.chars().take(500).collect::<String>()
            );
        }

        let tags_result: DockerHubTagsResult = serde_json::from_str(&response_text)
            .with_context(|| {
                format!(
                    "Failed to parse Docker Hub tags response. Status: {}, Response (first 500 chars): {}",
                    status,
                    response_text.chars().take(500).collect::<String>()
                )
            })?;

        Ok(tags_result
            .results
            .into_iter()
            .map(|tag| nexus_types::DockerImageTag {
                name: tag.name,
                last_updated: tag.last_updated,
                digest: tag.digest,
                size: tag.full_size,
            })
            .collect())
    }

    /// Download and save Docker image as tarball (tries Bollard API first, falls back to CLI)
    pub async fn download_image(
        &self,
        image: &str,
        registry_auth: Option<&nexus_types::RegistryAuth>,
        progress_tracker: crate::DownloadProgressTracker,
    ) -> Result<(PathBuf, String, i64)> {
        // Try bollard (Docker API) first for better progress tracking
        match self.download_image_with_bollard(image, registry_auth, progress_tracker.clone()).await {
            Ok(result) => {
                tracing::info!("Successfully downloaded {} using Docker API", image);
                return Ok(result);
            }
            Err(e) => {
                tracing::warn!(
                    "Bollard download failed for {}: {}. Falling back to CLI...",
                    image,
                    e
                );
                // Fall through to CLI implementation
            }
        }

        // Fallback to CLI-based download
        tracing::info!("Using CLI-based download for {}", image);
        self.download_image_with_cli(image, registry_auth, progress_tracker).await
    }

    /// Download image using CLI (fallback method)
    async fn download_image_with_cli(
        &self,
        image: &str,
        registry_auth: Option<&nexus_types::RegistryAuth>,
        progress_tracker: crate::DownloadProgressTracker,
    ) -> Result<(PathBuf, String, i64)> {
        // Create docker images directory if it doesn't exist
        let docker_dir = self.image_root.join("docker");
        
        // Try to create the directory, providing helpful error message if it fails
        if let Err(e) = tokio::fs::create_dir_all(&docker_dir).await {
            anyhow::bail!(
                "Failed to create docker images directory at {:?}: {}. \
                Hint: The image root directory ({:?}) may need write permissions. \
                Consider running with sudo, changing directory ownership, or setting MANAGER_IMAGE_ROOT to a writable path.",
                docker_dir,
                e,
                self.image_root
            );
        }

        // Login if authentication provided
        if let Some(auth) = registry_auth {
            let server = auth.server_address.as_deref().unwrap_or("https://index.docker.io/v1/");
            let status = Command::new("docker")
                .args([
                    "login",
                    server,
                    "-u", &auth.username,
                    "-p", &auth.password,
                ])
                .output()
                .await
                .context("Failed to execute docker login")?;

            if !status.status.success() {
                anyhow::bail!(
                    "Docker login failed: {}",
                    String::from_utf8_lossy(&status.stderr)
                );
            }
        }

        // Pull the image
        tracing::info!("Pulling Docker image: {}", image);

        // Update progress: Pulling
        {
            let mut progress_map = progress_tracker.lock().await;
            if let Some(progress) = progress_map.get_mut(image) {
                progress.status = "Pulling image layers...".to_string();
            }
        }

        // Use spawn to capture output (Docker outputs progress to stderr by default)
        let mut pull_process = Command::new("docker")
            .args(["pull", image])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn docker pull command")?;

        // Read stderr to capture output and errors
        // Docker outputs progress information to stderr (though not in easily parseable format)
        let stderr_handle = pull_process.stderr.take();
        let mut error_output = Vec::new();
        let image_name = image.to_string();

        if let Some(stderr) = stderr_handle {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();

            // Read all lines from stderr (progress + errors)
            while reader.read_line(&mut line).await.is_ok() && !line.is_empty() {
                // Store line for error messages
                error_output.push(line.clone());

                // Log progress lines for monitoring (Docker outputs progress to stderr)
                let trimmed = line.trim();

                // Update progress tracker based on docker output
                if trimmed.contains("Downloading") {
                    let mut progress_map = progress_tracker.lock().await;
                    if let Some(progress) = progress_map.get_mut(&image_name) {
                        progress.status = "Downloading layers...".to_string();
                    }
                } else if trimmed.contains("Extracting") {
                    let mut progress_map = progress_tracker.lock().await;
                    if let Some(progress) = progress_map.get_mut(&image_name) {
                        progress.status = "Extracting layers...".to_string();
                    }
                } else if trimmed.contains("Pull complete") {
                    let mut progress_map = progress_tracker.lock().await;
                    if let Some(progress) = progress_map.get_mut(&image_name) {
                        progress.status = "Pull complete, saving...".to_string();
                    }
                }

                if trimmed.contains("Downloading") || trimmed.contains("Extracting") || trimmed.contains("Pull complete") {
                    tracing::info!("Docker pull progress: {}", trimmed);
                }

                line.clear();
            }
        }

        let pull_status = pull_process.wait().await
            .context("Failed to wait for docker pull")?;

        if !pull_status.success() {
            let error_msg = if !error_output.is_empty() {
                error_output.join("")
            } else {
                format!("Docker pull exited with code: {}", pull_status.code().unwrap_or(-1))
            };
            
            tracing::error!("Docker pull failed for {}: {}", image, error_msg);
            anyhow::bail!("Docker pull failed: {}", error_msg);
        }
        
        tracing::info!("Successfully pulled Docker image: {}", image);

        // Sanitize image name for filename
        let safe_name = image
            .replace('/', "_")
            .replace(':', "_")
            .replace('.', "_");
        let tarball_path = docker_dir.join(format!("{}.tar", safe_name));

        // Save image as tarball
        tracing::info!("Saving Docker image to: {:?}", tarball_path);

        // Update progress: Saving
        {
            let mut progress_map = progress_tracker.lock().await;
            if let Some(progress) = progress_map.get_mut(image) {
                progress.status = "Saving as tarball...".to_string();
            }
        }

        let mut save_process = Command::new("docker")
            .args([
                "save",
                "-o",
                tarball_path.to_str().unwrap(),
                image,
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn docker save command")?;

        let save_status = save_process.wait().await
            .context("Failed to wait for docker save")?;

        if !save_status.success() {
            // Try to read stderr for error message
            let stderr = save_process.stderr.take();
            let error_msg = if let Some(mut stderr) = stderr {
                use tokio::io::AsyncReadExt;
                let mut buf = String::new();
                let _ = stderr.read_to_string(&mut buf).await;
                if !buf.is_empty() {
                    buf
                } else {
                    format!("Docker save exited with code: {}", save_status.code().unwrap_or(-1))
                }
            } else {
                format!("Docker save exited with code: {}", save_status.code().unwrap_or(-1))
            };
            
            tracing::error!("Docker save failed for {}: {}", image, error_msg);
            anyhow::bail!("Docker save failed: {}", error_msg);
        }
        
        tracing::info!("Successfully saved Docker image to: {:?}", tarball_path);

        // Get image inspect data for SHA256
        let inspect_output = Command::new("docker")
            .args(["inspect", image])
            .output()
            .await
            .context("Failed to execute docker inspect")?;

        let inspect_data: serde_json::Value = serde_json::from_slice(&inspect_output.stdout)
            .context("Failed to parse docker inspect output")?;

        let sha256 = inspect_data
            .get(0)
            .and_then(|v| v.get("Id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim_start_matches("sha256:")
            .to_string();

        // Get tarball size
        let metadata = tokio::fs::metadata(&tarball_path)
            .await
            .context("Failed to get tarball metadata")?;
        let size = metadata.len() as i64;

        Ok((tarball_path, sha256, size))
    }

    /// Download image using Bollard (Docker API) with real progress tracking
    async fn download_image_with_bollard(
        &self,
        image: &str,
        registry_auth: Option<&nexus_types::RegistryAuth>,
        progress_tracker: crate::DownloadProgressTracker,
    ) -> Result<(PathBuf, String, i64)> {
        tracing::info!("🔷 Attempting to download {} using Docker API (bollard)", image);

        // Connect to Docker daemon
        let docker = Docker::connect_with_local_defaults()
            .context("Failed to connect to Docker daemon - ensure Docker is running and socket is accessible")?;

        // Test connection
        docker.ping().await.context("Docker daemon not responding")?;
        tracing::info!("✅ Docker daemon connection successful");

        // Parse image name and tag
        let (image_name, tag) = if let Some(idx) = image.rfind(':') {
            (&image[..idx], &image[idx + 1..])
        } else {
            (image, "latest")
        };

        // Prepare authentication if provided
        let auth_config = registry_auth.map(|auth| bollard::auth::DockerCredentials {
            username: Some(auth.username.clone()),
            password: Some(auth.password.clone()),
            serveraddress: auth.server_address.clone(),
            ..Default::default()
        });

        // Create image pull options
        let options = CreateImageOptions {
            from_image: image_name,
            tag,
            ..Default::default()
        };

        // Start pulling the image
        let mut stream = docker.create_image(Some(options), None, auth_config);

        // Track total progress across all layers
        let mut layer_progress: std::collections::HashMap<String, (u64, u64)> = std::collections::HashMap::new();

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    // Log all status messages for debugging
                    if let Some(status) = &info.status {
                        tracing::debug!("Docker status: {} (id: {:?})", status, info.id);
                    }

                    // Update progress based on layer information
                    if let Some(id) = &info.id {
                        if let Some(progress_detail) = &info.progress_detail {
                            if let (Some(current), Some(total)) = (progress_detail.current, progress_detail.total) {
                                tracing::debug!("Layer {} progress: {} / {}", id, current, total);
                                layer_progress.insert(id.clone(), (current as u64, total as u64));
                            }
                        } else {
                            // Log when progress_detail is missing
                            if let Some(status) = &info.status {
                                if status.contains("Downloading") || status.contains("Extracting") {
                                    tracing::debug!("Status '{}' but no progress_detail for layer {}", status, id);
                                }
                            }
                        }
                    }

                    // Calculate total progress across all layers
                    let (total_current, total_total): (u64, u64) = layer_progress
                        .values()
                        .fold((0, 0), |(acc_curr, acc_tot), (curr, tot)| {
                            (acc_curr + curr, acc_tot + tot)
                        });

                    // Update progress tracker with real byte counts
                    {
                        let mut progress_map = progress_tracker.lock().await;
                        if let Some(progress) = progress_map.get_mut(image) {
                            let old_total = progress.total_bytes;

                            // Only update bytes if we have data
                            if total_total > 0 {
                                progress.current_bytes = total_current as i64;
                                progress.total_bytes = total_total as i64;

                                // Log when we first get total bytes
                                if old_total == 0 {
                                    tracing::info!(
                                        "📊 Download size determined: {} MB total",
                                        total_total / 1_000_000
                                    );
                                }
                            }

                            // Update status based on Docker's status message
                            if let Some(status) = &info.status {
                                match status.as_str() {
                                    "Pulling fs layer" => progress.status = "Pulling layers...".to_string(),
                                    "Downloading" => {
                                        progress.status = "Downloading layers...".to_string();
                                        // Force update even if no progress_detail
                                        if total_total == 0 {
                                            tracing::warn!("Downloading but no size information available yet");
                                        }
                                    },
                                    "Extracting" => progress.status = "Extracting layers...".to_string(),
                                    "Pull complete" => progress.status = "Pull complete".to_string(),
                                    "Already exists" => {
                                        progress.status = "Using cached layers...".to_string();
                                        tracing::info!("Layer already exists (cached)");
                                    },
                                    "Download complete" => progress.status = "Download complete".to_string(),
                                    "Status: Downloaded newer image" => progress.status = "Download complete".to_string(),
                                    _ => {}
                                }
                            }
                        }
                    }

                    // Log progress for monitoring (every 10% to avoid spam)
                    if let Some(status) = &info.status {
                        if total_total > 0 {
                            let percentage = (total_current as f64 / total_total as f64 * 100.0) as u32;
                            if percentage % 10 == 0 && total_current > 0 {
                                tracing::info!(
                                    "📦 Docker pull progress: {} - {}% ({} MB / {} MB)",
                                    status,
                                    percentage,
                                    total_current / 1_000_000,
                                    total_total / 1_000_000
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Docker pull error: {}", e);
                    anyhow::bail!("Failed to pull image: {}", e);
                }
            }
        }

        tracing::info!("Successfully pulled Docker image: {}", image);

        // Update progress: Saving
        {
            let mut progress_map = progress_tracker.lock().await;
            if let Some(progress) = progress_map.get_mut(image) {
                progress.status = "Saving as tarball...".to_string();
            }
        }

        // Create docker images directory if it doesn't exist
        let docker_dir = self.image_root.join("docker");
        tokio::fs::create_dir_all(&docker_dir)
            .await
            .context("Failed to create docker images directory")?;

        // Sanitize image name for filename
        let safe_name = image
            .replace('/', "_")
            .replace(':', "_")
            .replace('.', "_");
        let tarball_path = docker_dir.join(format!("{}.tar", safe_name));

        // Export image as tarball using bollard
        tracing::info!("Exporting image to tarball: {:?}", tarball_path);

        // Note: Bollard doesn't have a direct "save" equivalent yet
        // We'll fall back to CLI for this part
        let mut save_process = Command::new("docker")
            .args([
                "save",
                "-o",
                tarball_path.to_str().unwrap(),
                image,
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn docker save command")?;

        let save_status = save_process.wait().await
            .context("Failed to wait for docker save")?;

        if !save_status.success() {
            anyhow::bail!("Docker save failed with code: {}", save_status.code().unwrap_or(-1));
        }

        // Get image inspect data for SHA256
        let inspect_result = docker.inspect_image(image).await
            .context("Failed to inspect image")?;

        let sha256 = inspect_result.id
            .unwrap_or_default()
            .trim_start_matches("sha256:")
            .to_string();

        // Get tarball size
        let metadata = tokio::fs::metadata(&tarball_path)
            .await
            .context("Failed to get tarball metadata")?;
        let size = metadata.len() as i64;

        Ok((tarball_path, sha256, size))
    }

    /// Load Docker image tarball into a Docker daemon
    pub async fn load_image(&self, tarball_path: &PathBuf) -> Result<()> {
        tracing::info!("Loading Docker image from: {:?}", tarball_path);

        let output = Command::new("docker")
            .args(["load", "-i", tarball_path.to_str().unwrap()])
            .output()
            .await
            .context("Failed to execute docker load")?;

        if !output.status.success() {
            anyhow::bail!(
                "Docker load failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }
}
