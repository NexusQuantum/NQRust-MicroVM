use crate::features::sso::{crypto, oidc, provisioning, saml};
use crate::AppState;
use axum::{
    extract::{Form, Path, Query},
    http::StatusCode,
    response::{Html, Redirect},
    Extension, Json,
};
use nexus_types::{
    CreateSsoProviderRequest, ListSsoProviderConfigsResponse, ListSsoProvidersResponse,
    SsoProvider, SsoProviderConfig, SsoProviderPathParams, SsoSlugPathParams, SsoTestResult,
    UpdateSsoProviderRequest,
};
use serde::Deserialize;
use tracing::{error, info, warn};

// ─── Public Routes ─────────────────────────────────────────────────

/// List enabled SSO providers for the login page.
pub async fn list_enabled_providers(
    Extension(st): Extension<AppState>,
) -> Result<Json<ListSsoProvidersResponse>, StatusCode> {
    let providers = st.sso_providers.list_enabled().await.map_err(|e| {
        error!(?e, "failed to list SSO providers");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let items: Vec<SsoProvider> = providers
        .iter()
        .map(|p| SsoProvider {
            slug: p.slug.clone(),
            name: p.name.clone(),
            protocol: if p.protocol == "saml" {
                nexus_types::SsoProtocol::Saml
            } else {
                nexus_types::SsoProtocol::Oidc
            },
            icon_hint: p.icon_hint.clone(),
            display_order: p.display_order,
        })
        .collect();

    Ok(Json(ListSsoProvidersResponse { providers: items }))
}

#[derive(Deserialize)]
pub struct LoginQuery {
    redirect_after: Option<String>,
}

/// Initiate OIDC login — redirect to IdP.
pub async fn oidc_login_initiate(
    Extension(st): Extension<AppState>,
    Path(SsoSlugPathParams { slug }): Path<SsoSlugPathParams>,
    Query(query): Query<LoginQuery>,
) -> Result<Redirect, StatusCode> {
    let provider = st.sso_providers.get_by_slug(&slug).await.map_err(|_| {
        error!(slug = %slug, "SSO provider not found");
        StatusCode::NOT_FOUND
    })?;

    if provider.protocol != "oidc" {
        return Err(StatusCode::BAD_REQUEST);
    }

    let callback_url = format!("{}/v1/sso/oidc/{}/callback", st.sso_base_url, slug);
    let (auth_url, state_token, nonce_value, pkce_secret) =
        oidc::initiate_login(&provider, &callback_url, &st.sso_encryption_key)
            .await
            .map_err(|e| {
                error!(?e, "failed to initiate OIDC login");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    // Encrypt PKCE verifier for storage
    let pkce_encrypted = crypto::encrypt(&pkce_secret, &st.sso_encryption_key).map_err(|e| {
        error!(?e, "failed to encrypt PKCE verifier");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Store state
    st.auth_states
        .create_state(
            provider.id,
            &state_token,
            Some(&pkce_encrypted),
            Some(&nonce_value),
            query.redirect_after.as_deref(),
        )
        .await
        .map_err(|e| {
            error!(?e, "failed to store auth state");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!(slug = %slug, "OIDC login initiated");
    Ok(Redirect::temporary(auth_url.as_str()))
}

#[derive(Deserialize)]
pub struct OidcCallbackQuery {
    code: String,
    state: String,
}

/// OIDC callback — exchange code, provision user, redirect to frontend.
pub async fn oidc_callback(
    Extension(st): Extension<AppState>,
    Path(SsoSlugPathParams { slug }): Path<SsoSlugPathParams>,
    Query(query): Query<OidcCallbackQuery>,
) -> Result<Redirect, StatusCode> {
    // 1. Consume state
    let auth_state = st
        .auth_states
        .consume_state(&query.state)
        .await
        .map_err(|_| {
            warn!("invalid or expired OIDC state");
            StatusCode::BAD_REQUEST
        })?;

    // 2. Load provider
    let provider = st
        .sso_providers
        .get_by_slug(&slug)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // 3. Decrypt PKCE verifier
    let pkce_secret = crypto::decrypt(
        auth_state.pkce_verifier_encrypted.as_deref().unwrap_or(""),
        &st.sso_encryption_key,
    )
    .map_err(|e| {
        error!(?e, "failed to decrypt PKCE verifier");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // 4. Exchange code
    let callback_url = format!("{}/v1/sso/oidc/{}/callback", st.sso_base_url, slug);
    let role_claim = provider.role_claim_name.as_deref().unwrap_or("groups");

    let claims = oidc::exchange_code(
        &provider,
        &callback_url,
        &st.sso_encryption_key,
        &query.code,
        &pkce_secret,
        role_claim,
    )
    .await
    .map_err(|e| {
        error!(?e, "OIDC code exchange failed");
        StatusCode::UNAUTHORIZED
    })?;

    // 5. Find or provision user
    let result = provisioning::find_or_provision_user(
        &st.users,
        &st.user_identities,
        &provider,
        &claims.sub,
        claims.email.as_deref(),
        claims.preferred_username.as_deref(),
        claims.name.as_deref(),
        &claims.groups,
        Some(claims.raw.clone()),
    )
    .await
    .map_err(|e| {
        error!(?e, "SSO user provisioning failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // 6. Create internal bearer token
    let token = st
        .users
        .create_token(result.user.id, None)
        .await
        .map_err(|e| {
            error!(?e, "failed to create token");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // 7. Redirect to frontend callback
    let user_json = serde_json::to_string(&result.user.to_user()).unwrap_or_default();
    let redirect_url = format!(
        "{}/sso/callback?token={}&user={}",
        st.sso_frontend_url,
        urlencoding::encode(&token),
        urlencoding::encode(&user_json),
    );

    info!(
        user_id = %result.user.id,
        username = %result.user.username,
        was_created = result.was_created,
        "OIDC login successful"
    );

    Ok(Redirect::temporary(&redirect_url))
}

/// Initiate SAML login — redirect to IdP.
pub async fn saml_login_initiate(
    Extension(st): Extension<AppState>,
    Path(SsoSlugPathParams { slug }): Path<SsoSlugPathParams>,
) -> Result<Redirect, StatusCode> {
    let provider = st
        .sso_providers
        .get_by_slug(&slug)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    if provider.protocol != "saml" {
        return Err(StatusCode::BAD_REQUEST);
    }

    let redirect_url = saml::create_authn_request(&provider, &st.sso_base_url).map_err(|e| {
        error!(?e, "failed to create SAML AuthnRequest");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!(slug = %slug, "SAML login initiated");
    Ok(Redirect::temporary(&redirect_url))
}

/// SAML SP metadata endpoint.
pub async fn saml_metadata(
    Extension(st): Extension<AppState>,
    Path(SsoSlugPathParams { slug }): Path<SsoSlugPathParams>,
) -> Result<(StatusCode, [(String, String); 1], String), StatusCode> {
    let provider = st
        .sso_providers
        .get_by_slug(&slug)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let xml = saml::generate_sp_metadata(&provider, &st.sso_base_url).map_err(|e| {
        error!(?e, "failed to generate SAML metadata");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((
        StatusCode::OK,
        [("Content-Type".to_string(), "application/xml".to_string())],
        xml,
    ))
}

#[derive(Deserialize)]
pub struct SamlAcsForm {
    #[serde(rename = "SAMLResponse")]
    saml_response: String,
    #[serde(rename = "RelayState", default)]
    _relay_state: Option<String>,
}

/// SAML ACS endpoint — process SAML response, provision user, redirect to frontend.
pub async fn saml_acs(
    Extension(st): Extension<AppState>,
    Path(SsoSlugPathParams { slug }): Path<SsoSlugPathParams>,
    Form(form): Form<SamlAcsForm>,
) -> Result<Html<String>, StatusCode> {
    let provider = st
        .sso_providers
        .get_by_slug(&slug)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let role_claim = provider.role_claim_name.as_deref().unwrap_or("groups");

    let claims =
        saml::process_response(&provider, &form.saml_response, &st.sso_base_url, role_claim)
            .map_err(|e| {
                error!(?e, "SAML response processing failed");
                StatusCode::UNAUTHORIZED
            })?;

    // Derive username from claims
    let preferred_username = claims
        .attributes
        .get("preferred_username")
        .or_else(|| {
            claims
                .attributes
                .get("http://schemas.xmlsoap.org/ws/2005/05/identity/claims/name")
        })
        .and_then(|v| v.first().cloned());

    let result = provisioning::find_or_provision_user(
        &st.users,
        &st.user_identities,
        &provider,
        &claims.name_id,
        claims.email.as_deref(),
        preferred_username.as_deref(),
        claims.display_name.as_deref(),
        &claims.groups,
        Some(serde_json::to_value(&claims.attributes).unwrap_or_default()),
    )
    .await
    .map_err(|e| {
        error!(?e, "SSO user provisioning failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let token = st
        .users
        .create_token(result.user.id, None)
        .await
        .map_err(|e| {
            error!(?e, "failed to create token");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let user_json = serde_json::to_string(&result.user.to_user()).unwrap_or_default();

    info!(
        user_id = %result.user.id,
        username = %result.user.username,
        was_created = result.was_created,
        "SAML login successful"
    );

    // Return HTML that redirects to the frontend callback
    // (POST responses can't use 302 reliably for cross-origin)
    let html = format!(
        r#"<!DOCTYPE html>
<html><head><title>Redirecting...</title></head>
<body>
<p>Signing in...</p>
<script>
window.location.replace("{}/sso/callback?token={}&user={}");
</script>
</body></html>"#,
        st.sso_frontend_url,
        urlencoding::encode(&token),
        urlencoding::encode(&user_json),
    );

    Ok(Html(html))
}

// ─── Admin Routes ──────────────────────────────────────────────────

fn row_to_config(row: &crate::features::sso::repo::SsoProviderRow) -> SsoProviderConfig {
    SsoProviderConfig {
        id: row.id,
        name: row.name.clone(),
        slug: row.slug.clone(),
        protocol: if row.protocol == "saml" {
            nexus_types::SsoProtocol::Saml
        } else {
            nexus_types::SsoProtocol::Oidc
        },
        enabled: row.enabled,
        oidc_issuer_url: row.oidc_issuer_url.clone(),
        oidc_client_id: row.oidc_client_id.clone(),
        oidc_secret_set: row.oidc_client_secret_encrypted.is_some(),
        oidc_scopes: row.oidc_scopes.clone(),
        saml_idp_entity_id: row.saml_idp_entity_id.clone(),
        saml_idp_sso_url: row.saml_idp_sso_url.clone(),
        saml_sp_entity_id: row.saml_sp_entity_id.clone(),
        role_mapping: row.role_mapping.clone().unwrap_or(serde_json::json!({})),
        role_claim_name: row.role_claim_name.clone(),
        default_role: row
            .default_role
            .parse()
            .unwrap_or(nexus_types::Role::Viewer),
        allow_jit_provisioning: row.allow_jit_provisioning,
        username_claim: row.username_claim.clone(),
        email_claim: row.email_claim.clone(),
        display_name_claim: row.display_name_claim.clone(),
        icon_hint: row.icon_hint.clone(),
        display_order: row.display_order,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

pub async fn admin_list_providers(
    Extension(st): Extension<AppState>,
) -> Result<Json<ListSsoProviderConfigsResponse>, StatusCode> {
    let providers = st.sso_providers.list_all().await.map_err(|e| {
        error!(?e, "failed to list SSO providers");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let items = providers.iter().map(row_to_config).collect();
    Ok(Json(ListSsoProviderConfigsResponse { items }))
}

pub async fn admin_create_provider(
    Extension(st): Extension<AppState>,
    Json(req): Json<CreateSsoProviderRequest>,
) -> Result<Json<SsoProviderConfig>, StatusCode> {
    let encrypted_secret = if let Some(secret) = &req.oidc_client_secret {
        Some(
            crypto::encrypt(secret, &st.sso_encryption_key).map_err(|e| {
                error!(?e, "failed to encrypt client secret");
                StatusCode::INTERNAL_SERVER_ERROR
            })?,
        )
    } else {
        None
    };

    let row = st
        .sso_providers
        .create(&req, encrypted_secret.as_deref())
        .await
        .map_err(|e| {
            error!(?e, "failed to create SSO provider");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!(provider_id = %row.id, slug = %row.slug, "SSO provider created");
    Ok(Json(row_to_config(&row)))
}

pub async fn admin_get_provider(
    Extension(st): Extension<AppState>,
    Path(SsoProviderPathParams { id }): Path<SsoProviderPathParams>,
) -> Result<Json<SsoProviderConfig>, StatusCode> {
    let row = st
        .sso_providers
        .get_by_id(id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(row_to_config(&row)))
}

pub async fn admin_update_provider(
    Extension(st): Extension<AppState>,
    Path(SsoProviderPathParams { id }): Path<SsoProviderPathParams>,
    Json(req): Json<UpdateSsoProviderRequest>,
) -> Result<Json<SsoProviderConfig>, StatusCode> {
    let encrypted_secret = if let Some(secret) = &req.oidc_client_secret {
        Some(
            crypto::encrypt(secret, &st.sso_encryption_key).map_err(|e| {
                error!(?e, "failed to encrypt client secret");
                StatusCode::INTERNAL_SERVER_ERROR
            })?,
        )
    } else {
        None
    };

    let row = st
        .sso_providers
        .update(id, &req, encrypted_secret.as_deref())
        .await
        .map_err(|e| {
            error!(?e, "failed to update SSO provider");
            match e {
                crate::features::sso::repo::SsoRepoError::ProviderNotFound => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    info!(provider_id = %id, "SSO provider updated");
    Ok(Json(row_to_config(&row)))
}

pub async fn admin_delete_provider(
    Extension(st): Extension<AppState>,
    Path(SsoProviderPathParams { id }): Path<SsoProviderPathParams>,
) -> Result<Json<nexus_types::OkResponse>, StatusCode> {
    st.sso_providers.delete(id).await.map_err(|e| {
        error!(?e, "failed to delete SSO provider");
        match e {
            crate::features::sso::repo::SsoRepoError::ProviderNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    })?;

    info!(provider_id = %id, "SSO provider deleted");
    Ok(Json(nexus_types::OkResponse::default()))
}

pub async fn admin_test_provider(
    Extension(st): Extension<AppState>,
    Path(SsoProviderPathParams { id }): Path<SsoProviderPathParams>,
) -> Result<Json<SsoTestResult>, StatusCode> {
    let provider = st
        .sso_providers
        .get_by_id(id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    if provider.protocol == "oidc" {
        match oidc::test_discovery(&provider, &st.sso_encryption_key).await {
            Ok(_) => Ok(Json(SsoTestResult {
                success: true,
                message: Some("OIDC discovery succeeded".to_string()),
                error: None,
            })),
            Err(e) => Ok(Json(SsoTestResult {
                success: false,
                message: None,
                error: Some(format!("OIDC discovery failed: {}", e)),
            })),
        }
    } else {
        // SAML: try to generate metadata
        match saml::generate_sp_metadata(&provider, &st.sso_base_url) {
            Ok(_) => Ok(Json(SsoTestResult {
                success: true,
                message: Some("SAML SP metadata generated successfully".to_string()),
                error: None,
            })),
            Err(e) => Ok(Json(SsoTestResult {
                success: false,
                message: None,
                error: Some(format!("SAML configuration error: {}", e)),
            })),
        }
    }
}
