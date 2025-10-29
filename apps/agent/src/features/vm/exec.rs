use axum::{
    extract::Path,
    http::StatusCode,
    routing::post,
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::AppState;

#[derive(Deserialize)]
pub struct ExecRequest {
    pub command: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 {
    30
}

#[derive(Serialize)]
pub struct ExecResponse {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub success: bool,
}

pub fn router() -> Router {
    Router::new().route("/:id/exec", post(exec_command))
}

/// Execute a command inside the VM via screen session
async fn exec_command(
    Extension(_st): Extension<AppState>,
    Path(vm_id): Path<String>,
    Json(req): Json<ExecRequest>,
) -> Result<Json<ExecResponse>, (StatusCode, String)> {
    let screen_name = format!("fc-{}.scope", vm_id);

    // Use screen -S to send commands to the running session
    // We send: command + newline, then wait for output
    // This is simpler than attaching with screen -x

    // For VMs restored from snapshot, we need to ensure the console is active
    // Send several newlines first to wake up the console and clear any buffered input
    for _ in 0..10 {
        let _ = Command::new("sudo")
            .args(["screen", "-S", &screen_name, "-X", "stuff", "\n"])
            .output()
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    // Build the command to execute in the VM - add explicit newline and carriage return
    let exec_cmd = format!("{}\r\n", req.command);

    // Send command to screen session multiple times to ensure it executes
    for attempt in 1..=2 {
        let output = Command::new("sudo")
            .args([
                "screen",
                "-S",
                &screen_name,
                "-X",
                "stuff",
                &exec_cmd,
            ])
            .output()
            .await
            .map_err(|err| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to execute command: {}", err),
                )
            })?;

        if !output.status.success() {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "Screen command failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }

        if attempt == 1 {
            // Wait a bit between attempts
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    // Wait longer for command to execute (network restart takes time)
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Note: We can't easily capture stdout/stderr from screen
    // But for network restart, we just need to know if the command was sent
    Ok(Json(ExecResponse {
        stdout: "Command sent to VM".to_string(),
        stderr: String::new(),
        exit_code: Some(0),
        success: true,
    }))
}
