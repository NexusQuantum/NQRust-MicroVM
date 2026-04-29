//! Thin async wrapper over aws-sdk-s3 for the backup pipeline.

use aws_credential_types::Credentials;
use aws_sdk_s3::{
    config::{Builder, Region},
    error::SdkError,
    operation::head_object::HeadObjectError,
    Client,
};
use std::time::Duration;

#[derive(Clone)]
pub struct BackupTargetConfig {
    pub endpoint: String,
    pub region: Option<String>,
    pub bucket: String,
    pub prefix: String,
    pub access_key_id: String,
    pub secret_access_key: String,
}

pub fn make_client(cfg: &BackupTargetConfig) -> Client {
    let creds = Credentials::new(
        cfg.access_key_id.clone(),
        cfg.secret_access_key.clone(),
        None,
        None,
        "nqrust-backup",
    );
    let region = Region::new(cfg.region.clone().unwrap_or_else(|| "us-east-1".into()));
    let cfg_built = Builder::new()
        .behavior_version_latest()
        .endpoint_url(&cfg.endpoint)
        .credentials_provider(creds)
        .region(region)
        .force_path_style(true)
        .timeout_config(
            aws_sdk_s3::config::timeout::TimeoutConfig::builder()
                .operation_timeout(Duration::from_secs(120))
                .build(),
        )
        .build();
    Client::from_conf(cfg_built)
}

#[derive(Debug, thiserror::Error)]
pub enum S3Error {
    #[error("s3: {0}")]
    Other(String),
}

pub async fn head_object(client: &Client, bucket: &str, key: &str) -> Result<bool, S3Error> {
    match client.head_object().bucket(bucket).key(key).send().await {
        Ok(_) => Ok(true),
        Err(SdkError::ServiceError(svc)) if matches!(svc.err(), HeadObjectError::NotFound(_)) => {
            Ok(false)
        }
        Err(e) => Err(S3Error::Other(format!("head: {e}"))),
    }
}

pub async fn put_object(
    client: &Client,
    bucket: &str,
    key: &str,
    body: Vec<u8>,
) -> Result<(), S3Error> {
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body.into())
        .send()
        .await
        .map_err(|e| S3Error::Other(format!("put: {e}")))?;
    Ok(())
}

pub async fn get_object(client: &Client, bucket: &str, key: &str) -> Result<Vec<u8>, S3Error> {
    let resp = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| S3Error::Other(format!("get: {e}")))?;
    let body = resp
        .body
        .collect()
        .await
        .map_err(|e| S3Error::Other(format!("get body: {e}")))?;
    Ok(body.into_bytes().to_vec())
}
