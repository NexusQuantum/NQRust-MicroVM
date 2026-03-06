use crate::features::licensing::license_service;
use crate::features::licensing::service;
use crate::features::users::audit;
use crate::features::users::middleware::get_client_ip;
use crate::features::users::repo::AuthenticatedUser;
use crate::AppState;
use axum::http::StatusCode;
use axum::{Extension, http::HeaderMap, Json};
use nexus_types::{
    AuditAction, EulaAcceptRequest, EulaAcceptResponse, EulaInfo, EulaStatus,
    LicenseActivateRequest, LicenseState, LicenseUploadRequest,
};

pub async fn get_eula_info() -> Result<Json<EulaInfo>, (StatusCode, String)> {
    Ok(Json(service::get_eula_info().await))
}

pub async fn get_eula_status(
    Extension(state): Extension<AppState>,
) -> Result<Json<EulaStatus>, (StatusCode, String)> {
    service::get_app_eula_status(&state.licensing)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get eula status: {}", e)))
}

pub async fn accept_eula(
    Extension(state): Extension<AppState>,
    Json(req): Json<EulaAcceptRequest>,
) -> Result<Json<EulaAcceptResponse>, (StatusCode, String)> {
    service::accept_app_eula(&state.licensing, req)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
        .map(|_| Json(EulaAcceptResponse { success: true }))
}

// ── License endpoints ──

pub async fn get_license_status(
    Extension(state): Extension<AppState>,
) -> Result<Json<LicenseState>, (StatusCode, String)> {
    let guard = state.license_state.read().await;
    Ok(Json(guard.clone()))
}

pub async fn activate_license(
    Extension(state): Extension<AppState>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    headers: HeaderMap,
    Json(req): Json<LicenseActivateRequest>,
) -> Result<Json<LicenseState>, (StatusCode, String)> {
    // Only admins can activate licenses
    if user.role != nexus_types::Role::Admin {
        return Err((
            StatusCode::FORBIDDEN,
            "Only administrators can activate licenses".to_string(),
        ));
    }

    let result = license_service::activate_license(
        &state.license_config,
        &state.licensing,
        &state.license_state,
        &req.license_key,
    )
    .await;

    // Audit log
    let client_ip = get_client_ip(&headers);
    let details = serde_json::json!({
        "status": result.status,
        "is_licensed": result.is_licensed,
    });
    let _ = audit::log_success(
        &state.db,
        user.id,
        &user.username,
        AuditAction::ActivateLicense,
        None,
        None,
        Some(details),
        client_ip.as_deref(),
    )
    .await;

    Ok(Json(result))
}

pub async fn activate_license_file(
    Extension(state): Extension<AppState>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    headers: HeaderMap,
    Json(req): Json<LicenseUploadRequest>,
) -> Result<Json<LicenseState>, (StatusCode, String)> {
    if user.role != nexus_types::Role::Admin {
        return Err((
            StatusCode::FORBIDDEN,
            "Only administrators can activate licenses".to_string(),
        ));
    }

    let result = license_service::activate_offline_license(
        &state.license_config,
        &state.licensing,
        &state.license_state,
        &req.file_content,
    )
    .await;

    let client_ip = get_client_ip(&headers);
    let details = serde_json::json!({
        "status": result.status,
        "is_licensed": result.is_licensed,
        "method": "offline_lic",
    });
    let _ = audit::log_success(
        &state.db,
        user.id,
        &user.username,
        AuditAction::ActivateLicense,
        None,
        None,
        Some(details),
        client_ip.as_deref(),
    )
    .await;

    Ok(Json(result))
}

