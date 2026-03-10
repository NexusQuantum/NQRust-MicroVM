use crate::features::licensing::repo::LicensingRepository;
use crate::features::licensing::{AVAILABLE_LANGUAGES, CURRENT_EULA_VERSION};
use nexus_types::{EulaAcceptRequest, EulaInfo, EulaStatus};

pub async fn get_eula_info() -> EulaInfo {
    EulaInfo {
        version: CURRENT_EULA_VERSION.to_string(),
        languages: AVAILABLE_LANGUAGES.iter().map(|&s| s.to_string()).collect(),
    }
}

pub async fn get_app_eula_status(repo: &LicensingRepository) -> Result<EulaStatus, sqlx::Error> {
    let latest = repo.get_app_acceptance().await?;
    let needs_acceptance = match &latest {
        Some(v) => v != CURRENT_EULA_VERSION,
        None => true,
    };
    Ok(EulaStatus {
        needs_acceptance,
        latest_accepted_version: latest,
    })
}

pub async fn accept_app_eula(
    repo: &LicensingRepository,
    req: EulaAcceptRequest,
) -> Result<(), &'static str> {
    if req.version != CURRENT_EULA_VERSION {
        return Err("Invalid version: must accept the current EULA version");
    }
    if !AVAILABLE_LANGUAGES.contains(&req.language.as_str()) {
        return Err("Invalid language provided");
    }
    repo.record_app_acceptance(&req.version, &req.language)
        .await
        .map_err(|_| "Database error while recording acceptance")
}
