/// Fast VM provisioning using btrfs reflinks and host-side optimizations
///
/// This module provides optimized provisioning that:
/// 1. Uses btrfs reflink for instant rootfs copies (if available)
/// 2. Detects guest IP from host neighbor table (no waiting for guest-agent)
/// 3. Uses smart exponential backoff for all polling
/// 4. Supports snapshot-based provisioning (future)
use anyhow::{Context, Result};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Detect if /srv/images is on btrfs (supports reflinks)
pub async fn is_btrfs_available() -> Result<bool> {
    use tokio::process::Command;

    let output = Command::new("df")
        .args(["-T", "/srv/images"])
        .output()
        .await
        .context("Failed to check filesystem type")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.contains("btrfs"))
}

/// Fast copy using btrfs reflink (instant COW)
/// Falls back to regular copy if not on btrfs
pub async fn reflink_copy(src: &str, dest: &str) -> Result<()> {
    use tokio::process::Command;

    eprintln!("[FastCopy] Attempting reflink copy: {} -> {}", src, dest);

    // Try reflink first
    let result = Command::new("cp")
        .args(["--reflink=always", src, dest])
        .status()
        .await;

    match result {
        Ok(status) if status.success() => {
            eprintln!("[FastCopy] ✅ Reflink copy succeeded (instant COW)");
            Ok(())
        }
        _ => {
            eprintln!("[FastCopy] ⚠️  Reflink not available, falling back to regular copy");
            let fallback = Command::new("cp")
                .args([src, dest])
                .status()
                .await
                .context("Failed to execute fallback cp")?;

            if fallback.success() {
                Ok(())
            } else {
                anyhow::bail!("Failed to copy {} to {}", src, dest)
            }
        }
    }
}

/// Detect guest IP from host-side neighbor table
/// This is much faster than waiting for guest-agent reporting
/// Returns only NEW IPs that appear after this function is called (not pre-existing ones)
pub async fn detect_ip_from_neighbor_table(
    _tap_name: &str,
    bridge_name: &str,
    timeout_secs: u64,
) -> Result<Option<String>> {
    use tokio::process::Command;
    use std::collections::HashSet;

    eprintln!(
        "[FastIP] Monitoring neighbor table for NEW IP on bridge {}",
        bridge_name
    );

    let start = Instant::now();
    let deadline = start + Duration::from_secs(timeout_secs);

    // First, get the list of existing IPs (baseline)
    let output = Command::new("ip")
        .args(["neigh", "show", "dev", bridge_name])
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut existing_ips: HashSet<String> = HashSet::new();
    
    for line in stdout.lines() {
        if let Some(ip) = line.split_whitespace().next() {
            if !ip.is_empty() && ip != "127.0.0.1" {
                existing_ips.insert(ip.to_string());
            }
        }
    }

    eprintln!("[FastIP] Baseline: {} existing IPs in neighbor table", existing_ips.len());

    // Now wait for a NEW IP to appear
    let mut wait_ms = 100u64;

    while Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(wait_ms)).await;
        
        // Get current neighbor table
        let output = Command::new("ip")
            .args(["neigh", "show", "dev", bridge_name])
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Look for REACHABLE or STALE entries that are NEW
        for line in stdout.lines() {
            if line.contains("REACHABLE") || line.contains("STALE") {
                // Extract IP address (first field)
                if let Some(ip) = line.split_whitespace().next() {
                    // Check if this is a NEW IP (not in baseline)
                    if !ip.is_empty() && ip != "127.0.0.1" && !existing_ips.contains(ip) {
                        let elapsed = start.elapsed();
                        eprintln!(
                            "[FastIP] ✅ Detected NEW IP {} in {:.2}s",
                            ip,
                            elapsed.as_secs_f64()
                        );
                        return Ok(Some(ip.to_string()));
                    }
                }
            }
        }

        // Exponential backoff with cap
        wait_ms = std::cmp::min(wait_ms * 2, 3000); // Cap at 3s
    }

    eprintln!(
        "[FastIP] ⏱️  Timeout after {}s, no NEW IP detected",
        timeout_secs
    );
    Ok(None)
}

/// Smart polling with exponential backoff
/// Returns immediately on success, backs off exponentially on failure
pub async fn poll_with_backoff<F, Fut, T>(
    description: &str,
    timeout_secs: u64,
    check_fn: F,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<Option<T>>>,
{
    eprintln!("[SmartPoll] Starting: {}", description);

    let start = Instant::now();
    let deadline = start + Duration::from_secs(timeout_secs);

    // Exponential backoff: 50ms, 100ms, 200ms, 500ms, 1s, 2s, 3s...
    let mut wait_ms = 50u64;
    let mut attempt = 0;

    while Instant::now() < deadline {
        attempt += 1;

        match check_fn().await {
            Ok(Some(result)) => {
                let elapsed = start.elapsed();
                eprintln!(
                    "[SmartPoll] ✅ {} succeeded in {:.2}s (attempt {})",
                    description,
                    elapsed.as_secs_f64(),
                    attempt
                );
                return Ok(result);
            }
            Ok(None) => {
                // Continue polling
            }
            Err(e) => {
                if attempt % 10 == 0 {
                    eprintln!(
                        "[SmartPoll] Attempt {} failed: {} (retrying...)",
                        attempt, e
                    );
                }
            }
        }

        // Exponential backoff with cap
        tokio::time::sleep(Duration::from_millis(wait_ms)).await;
        wait_ms = std::cmp::min(wait_ms * 2, 3000); // Cap at 3s
    }

    anyhow::bail!(
        "{} timed out after {} seconds",
        description,
        timeout_secs
    )
}

/// Fast guest IP detection combining multiple strategies
/// 1. Check host neighbor table (fast, no guest-agent needed)
/// 2. Poll VM guest_ip field (updated by guest-agent)
/// 3. Return as soon as either succeeds
pub async fn fast_detect_guest_ip(
    db: &sqlx::PgPool,
    vm_id: Uuid,
    bridge_name: &str,
    tap_name: &str,
    timeout_secs: u64,
) -> Result<String> {
    eprintln!("[FastDetect] Starting parallel IP detection for VM {}", vm_id);

    let start = Instant::now();
    let deadline = start + Duration::from_secs(timeout_secs);

    // Try both strategies in parallel with exponential backoff
    let mut wait_ms = 100u64;
    let mut attempt = 0;

    while Instant::now() < deadline {
        attempt += 1;
        
        // Strategy 1: Check neighbor table (host-side, faster)
        // For snapshot-restored VMs, give a small timeout for DHCP to complete
        let remaining_secs = (deadline.saturating_duration_since(Instant::now())).as_secs();
        let neighbor_timeout = if attempt == 1 { 2 } else { 0 }; // First check waits 2s for DHCP
        
        if let Ok(Some(ip)) = detect_ip_from_neighbor_table(tap_name, bridge_name, neighbor_timeout).await {
            let elapsed = start.elapsed();
            eprintln!(
                "[FastDetect] ✅ Found IP via neighbor table: {} in {:.2}s",
                ip,
                elapsed.as_secs_f64()
            );
            return Ok(ip);
        }

        // Strategy 2: Check VM record (guest-agent reported)
        if let Ok(vm) = crate::features::vms::repo::get(db, vm_id).await {
            if let Some(ip) = vm.guest_ip {
                let elapsed = start.elapsed();
                eprintln!(
                    "[FastDetect] ✅ Found IP via guest-agent: {} in {:.2}s",
                    ip,
                    elapsed.as_secs_f64()
                );
                return Ok(ip);
            }
        }

        // Log progress periodically
        if attempt % 10 == 0 {
            eprintln!(
                "[FastDetect] Still waiting for IP... ({}s elapsed, {}s remaining)",
                start.elapsed().as_secs(),
                remaining_secs
            );
        }

        // Exponential backoff
        tokio::time::sleep(Duration::from_millis(wait_ms)).await;
        wait_ms = std::cmp::min(wait_ms * 2, 2000); // Cap at 2s
    }

    anyhow::bail!(
        "Failed to detect guest IP for VM {} after {}s",
        vm_id,
        timeout_secs
    )
}

/// Check if runtime is ready with aggressive timeout
pub async fn fast_runtime_check(guest_ip: &str, port: u16, timeout_secs: u64) -> Result<()> {
    let url = format!("http://{}:{}/health", guest_ip, port);

    eprintln!("[FastRuntime] Checking runtime at {}", url);

    poll_with_backoff("Runtime health check", timeout_secs, || {
        let url = url.clone();
        async move {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()?;

            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => Ok(Some(())),
                _ => Ok(None),
            }
        }
    })
    .await
}

/// Check if Docker daemon is ready with aggressive timeout
pub async fn fast_docker_check(guest_ip: &str, timeout_secs: u64) -> Result<()> {
    let url = format!("http://{}:2375/_ping", guest_ip);

    eprintln!("[FastDocker] Checking Docker daemon at {}", url);

    poll_with_backoff("Docker daemon check", timeout_secs, || {
        let url = url.clone();
        async move {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()?;

            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => Ok(Some(())),
                _ => Ok(None),
            }
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_btrfs_detection() {
        // Just ensure it doesn't crash
        let _ = is_btrfs_available().await;
    }

    #[tokio::test]
    async fn test_backoff_timing() {
        // Test that backoff actually waits
        let start = Instant::now();

        let result = poll_with_backoff("test", 2, || async { Ok::<Option<()>, anyhow::Error>(None) }).await;

        assert!(result.is_err());
        assert!(start.elapsed().as_secs() >= 2);
    }
}
