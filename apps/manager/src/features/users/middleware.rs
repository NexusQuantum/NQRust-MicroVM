use axum::{
    extract::Request,
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
    Extension,
};
use crate::AppState;
use crate::features::users::repo::{AuthenticatedUser, UserRepoError};
use nexus_types::Role;

pub async fn auth_middleware(
    Extension(st): Extension<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Allow /login endpoint without authentication
    if req.uri().path().ends_with("/login") {
        return Ok(next.run(req).await);
    }

    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = auth_header.strip_prefix("Bearer ").unwrap().trim();
    if token.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let user = st
        .users
        .validate_token(token)
        .await
        .map_err(|e| match e {
            UserRepoError::InvalidToken | UserRepoError::TokenExpired => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    req.extensions_mut().insert(user);
    Ok(next.run(req).await)
}

/// Middleware to require admin role
pub async fn require_admin(
    Extension(user): Extension<AuthenticatedUser>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if user.role != Role::Admin {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(next.run(req).await)
}

/// Helper to extract client IP address from request
fn get_client_ip(req: &Request) -> Option<String> {
    // Try X-Forwarded-For header first (for proxied requests)
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            // Take the first IP if there are multiple
            return forwarded_str.split(',').next().map(|s| s.trim().to_string());
        }
    }

    // Try X-Real-IP header
    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return Some(ip_str.to_string());
        }
    }

    // TODO: Could extract from connection info if available
    None
}

