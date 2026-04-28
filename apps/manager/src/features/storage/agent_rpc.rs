#![allow(dead_code)]

use anyhow::{anyhow, Context, Result};
use nexus_storage::{AttachedPath, BackendKind, VolumeHandle};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn agent_url(host_addr: &str, path: &str) -> String {
    let base = if host_addr.starts_with("http") {
        host_addr.to_string()
    } else {
        format!("http://{host_addr}")
    };
    format!("{base}{path}")
}

#[derive(Serialize)]
struct AttachReq<'a> {
    volume: &'a VolumeHandle,
}

#[derive(Deserialize)]
struct AttachResp {
    attached: AttachedPath,
}

pub async fn agent_attach(host_addr: &str, volume: &VolumeHandle) -> Result<AttachedPath> {
    let resp = Client::new()
        .post(agent_url(host_addr, "/v1/storage/attach"))
        .json(&AttachReq { volume })
        .send()
        .await
        .with_context(|| format!("POST /v1/storage/attach to {host_addr}"))?;
    if !resp.status().is_success() {
        let s = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("agent attach: {s}: {body}"));
    }
    Ok(resp.json::<AttachResp>().await?.attached)
}

#[derive(Serialize)]
struct DetachReq<'a> {
    volume: &'a VolumeHandle,
    attached: &'a AttachedPath,
}

pub async fn agent_detach(
    host_addr: &str,
    volume: &VolumeHandle,
    attached: &AttachedPath,
) -> Result<()> {
    let resp = Client::new()
        .post(agent_url(host_addr, "/v1/storage/detach"))
        .json(&DetachReq { volume, attached })
        .send()
        .await
        .with_context(|| format!("POST /v1/storage/detach to {host_addr}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("agent detach: {}", resp.status()));
    }
    Ok(())
}

#[derive(Serialize)]
struct PopulateReq<'a> {
    backend_kind: BackendKind,
    attached: &'a AttachedPath,
    source_path: &'a PathBuf,
    target_size_bytes: u64,
}

pub async fn agent_populate(
    host_addr: &str,
    backend_kind: BackendKind,
    attached: &AttachedPath,
    source_path: &PathBuf,
    target_size_bytes: u64,
) -> Result<()> {
    let resp = Client::new()
        .post(agent_url(host_addr, "/v1/storage/populate"))
        .json(&PopulateReq {
            backend_kind,
            attached,
            source_path,
            target_size_bytes,
        })
        .send()
        .await
        .with_context(|| format!("POST /v1/storage/populate to {host_addr}"))?;
    if !resp.status().is_success() {
        let s = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("agent populate: {s}: {body}"));
    }
    Ok(())
}

#[derive(Serialize)]
struct Resize2fsReq<'a> {
    attached: &'a AttachedPath,
}

pub async fn agent_resize2fs(host_addr: &str, attached: &AttachedPath) -> Result<()> {
    let resp = Client::new()
        .post(agent_url(host_addr, "/v1/storage/resize2fs"))
        .json(&Resize2fsReq { attached })
        .send()
        .await
        .with_context(|| format!("POST /v1/storage/resize2fs to {host_addr}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("agent resize2fs: {}", resp.status()));
    }
    Ok(())
}
