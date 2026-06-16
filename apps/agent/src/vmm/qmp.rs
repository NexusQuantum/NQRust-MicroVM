//! Minimal QEMU Machine Protocol (QMP) client over a Unix domain socket.
//!
//! QMP is a JSON-RPC dialect: newline-terminated JSON objects exchanged over
//! the QEMU control socket. On connect, QEMU sends a greeting, then expects
//! `{"execute": "qmp_capabilities"}` before any other command.
//!
//! This client is deliberately small — it only covers what the agent needs:
//! capability handshake, plain `execute`-style commands, and event drain.

use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::timeout;

const QMP_READ_TIMEOUT: Duration = Duration::from_secs(30);
const QMP_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// QMP client wrapping a connected UDS.
pub struct QmpClient {
    reader: BufReader<tokio::net::unix::OwnedReadHalf>,
    writer: tokio::net::unix::OwnedWriteHalf,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct QmpGreeting {
    #[serde(rename = "QMP")]
    pub qmp: QmpGreetingBody,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct QmpGreetingBody {
    pub version: serde_json::Value,
    pub capabilities: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct Command<'a, T: Serialize> {
    execute: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    arguments: Option<T>,
}

impl QmpClient {
    /// Connect to a QMP socket and run the capability handshake.
    pub async fn connect(sock: &Path) -> Result<Self> {
        let stream = UnixStream::connect(sock)
            .await
            .with_context(|| format!("connect QMP socket {}", sock.display()))?;
        let (r, w) = stream.into_split();
        let mut client = QmpClient {
            reader: BufReader::new(r),
            writer: w,
        };
        let _greet: QmpGreeting = timeout(QMP_HANDSHAKE_TIMEOUT, client.read_message())
            .await
            .context("timed out waiting for QMP greeting")??;
        client
            .execute::<serde_json::Value>("qmp_capabilities", None)
            .await
            .context("QMP capability negotiation failed")?;
        Ok(client)
    }

    /// Execute a QMP command. `arguments` is optional. Returns the `return`
    /// field of the response on success. Drains async events that arrive
    /// before the command's response.
    pub async fn execute<A: Serialize>(
        &mut self,
        command: &str,
        arguments: Option<A>,
    ) -> Result<serde_json::Value> {
        let req = Command {
            execute: command,
            arguments,
        };
        let mut line = serde_json::to_vec(&req)?;
        line.push(b'\n');
        self.writer.write_all(&line).await?;
        self.writer.flush().await?;

        loop {
            let msg: serde_json::Value = timeout(QMP_READ_TIMEOUT, self.read_message())
                .await
                .with_context(|| format!("QMP {} timed out", command))??;
            if let Some(ret) = msg.get("return") {
                return Ok(ret.clone());
            }
            if let Some(err) = msg.get("error") {
                return Err(anyhow!("QMP {} error: {}", command, err));
            }
            // event / async notification — drain and keep reading
            if msg.get("event").is_some() {
                tracing::trace!(?msg, "QMP event drained");
                continue;
            }
            // Unknown message shape — log and keep waiting
            tracing::warn!(?msg, "unexpected QMP message shape, ignoring");
        }
    }

    async fn read_message<T: serde::de::DeserializeOwned>(&mut self) -> Result<T> {
        let mut buf = String::new();
        let n = self.reader.read_line(&mut buf).await?;
        if n == 0 {
            return Err(anyhow!("QMP socket closed by QEMU"));
        }
        Ok(serde_json::from_str(buf.trim())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;
    use tokio::net::UnixListener;

    #[tokio::test]
    async fn handshake_and_execute_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("qmp.sock");
        let sock_for_server = sock.clone();
        let listener = UnixListener::bind(&sock).unwrap();

        let server = tokio::spawn(async move {
            let _ = sock_for_server;
            let (stream, _) = listener.accept().await.unwrap();
            let (read_half, mut write_half) = stream.into_split();
            // 1. Send greeting
            write_half
                .write_all(
                    b"{\"QMP\":{\"version\":{\"qemu\":{\"major\":11,\"minor\":0,\"micro\":0}},\"capabilities\":[]}}\n",
                )
                .await
                .unwrap();
            let mut reader = tokio::io::BufReader::new(read_half);
            let mut line = String::new();
            // 2. Read capabilities negotiation, respond with return:{}
            reader.read_line(&mut line).await.unwrap();
            assert!(line.contains("qmp_capabilities"));
            write_half.write_all(b"{\"return\":{}}\n").await.unwrap();
            // 3. Read application command
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            assert!(line.contains("query-status"));
            write_half
                .write_all(b"{\"return\":{\"status\":\"running\",\"running\":true}}\n")
                .await
                .unwrap();
        });

        let mut client = QmpClient::connect(&sock).await.unwrap();
        let ret = client
            .execute::<serde_json::Value>("query-status", None)
            .await
            .unwrap();
        assert_eq!(ret["status"], "running");
        server.await.unwrap();
    }
}
