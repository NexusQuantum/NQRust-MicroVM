use std::io::{self, Write};

use anyhow::{Context, Result};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use uuid::Uuid;

use crate::client::Client;

pub async fn connect_vm_shell(client: &Client, id: Uuid, show_credentials: bool) -> Result<()> {
    if show_credentials {
        print_credentials(client, id).await;
    }

    eprintln!("Connecting to VM shell. Detach with Ctrl+].");
    let _raw = RawModeGuard::enter().context("enabling raw terminal mode")?;

    let ws_url = client.ws_url(&format!("/v1/vms/{id}/shell/ws"));
    let (stream, _) = connect_async(&ws_url)
        .await
        .with_context(|| format!("connecting to {ws_url}"))?;
    let (mut write, mut read) = stream.split();

    let input = async {
        let mut stdin = tokio::io::stdin();
        let mut buf = [0_u8; 1024];
        loop {
            let n = stdin.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            if let Some(pos) = buf[..n].iter().position(|byte| *byte == 0x1d) {
                if pos > 0 {
                    write.send(Message::Binary(buf[..pos].to_vec())).await?;
                }
                write.send(Message::Close(None)).await?;
                break;
            }
            write.send(Message::Binary(buf[..n].to_vec())).await?;
        }
        anyhow::Ok(())
    };

    let output = async {
        let mut stdout = tokio::io::stdout();
        while let Some(message) = read.next().await {
            match message? {
                Message::Text(text) => stdout.write_all(text.as_bytes()).await?,
                Message::Binary(bytes) => stdout.write_all(&bytes).await?,
                Message::Ping(_) | Message::Pong(_) => {}
                Message::Close(_) => break,
                _ => {}
            }
            stdout.flush().await?;
        }
        anyhow::Ok(())
    };

    tokio::select! {
        result = input => result?,
        result = output => result?,
    }

    Ok(())
}

async fn print_credentials(client: &Client, id: Uuid) {
    let Ok(value) = client.get::<Value>(&format!("/v1/vms/{id}/shell")).await else {
        return;
    };

    let username = value.get("username").and_then(Value::as_str);
    let password = value.get("password").and_then(Value::as_str);
    if let (Some(username), Some(password)) = (username, password) {
        eprintln!("Login credentials: username={username} password={password}");
    }
}

struct RawModeGuard;

impl RawModeGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = io::stdout().flush();
    }
}
