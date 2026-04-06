use crate::features::sso::crypto;
use crate::features::sso::repo::SsoProviderRow;
use anyhow::{Context, Result};
use base64::Engine;
use openidconnect::{
    core::{CoreProviderMetadata, CoreResponseType},
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use serde_json::Value;

/// Claims extracted from a successful OIDC authentication.
#[derive(Debug, Clone)]
pub struct OidcClaims {
    pub sub: String,
    pub email: Option<String>,
    pub preferred_username: Option<String>,
    pub name: Option<String>,
    pub groups: Vec<String>,
    pub raw: Value,
}

fn make_http_client() -> Result<reqwest::Client> {
    reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("failed to build HTTP client")
}

/// Generate an authorization URL with PKCE, state, and nonce. Returns
/// `(redirect_url, state_token, nonce_value, pkce_verifier)`.
pub async fn initiate_login(
    provider: &SsoProviderRow,
    callback_url: &str,
    encryption_key: &[u8; 32],
) -> Result<(url::Url, String, String, String)> {
    let http_client = make_http_client()?;
    let (client_id, client_secret, issuer_url) = extract_oidc_config(provider, encryption_key)?;

    let metadata = CoreProviderMetadata::discover_async(issuer_url, &http_client)
        .await
        .map_err(|e| anyhow::anyhow!("OIDC discovery failed: {}", e))?;

    let redirect_url =
        RedirectUrl::new(callback_url.to_string()).context("invalid redirect URL")?;

    let client =
        openidconnect::core::CoreClient::from_provider_metadata(metadata, client_id, client_secret)
            .set_redirect_uri(redirect_url);

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let nonce = Nonce::new(uuid::Uuid::new_v4().to_string());

    let scopes_str = provider
        .oidc_scopes
        .as_deref()
        .unwrap_or("openid profile email");
    let extra_scopes: Vec<Scope> = scopes_str
        .split_whitespace()
        .filter(|s| *s != "openid")
        .map(|s| Scope::new(s.to_string()))
        .collect();

    let nonce_clone = nonce.clone();
    let mut auth_request = client.authorize_url(
        AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
        CsrfToken::new_random,
        move || nonce_clone,
    );

    for scope in extra_scopes {
        auth_request = auth_request.add_scope(scope);
    }

    let (url, state, _nonce_out) = auth_request.set_pkce_challenge(pkce_challenge).url();

    Ok((
        url,
        state.secret().clone(),
        nonce.secret().clone(),
        pkce_verifier.secret().clone(),
    ))
}

/// Exchange an authorization code for tokens and extract claims.
pub async fn exchange_code(
    provider: &SsoProviderRow,
    callback_url: &str,
    encryption_key: &[u8; 32],
    code: &str,
    pkce_verifier_secret: &str,
    role_claim_name: &str,
) -> Result<OidcClaims> {
    let http_client = make_http_client()?;
    let (client_id, client_secret, issuer_url) = extract_oidc_config(provider, encryption_key)?;

    let metadata = CoreProviderMetadata::discover_async(issuer_url, &http_client)
        .await
        .map_err(|e| anyhow::anyhow!("OIDC discovery failed: {}", e))?;

    let redirect_url =
        RedirectUrl::new(callback_url.to_string()).context("invalid redirect URL")?;

    let client =
        openidconnect::core::CoreClient::from_provider_metadata(metadata, client_id, client_secret)
            .set_redirect_uri(redirect_url);

    let pkce_verifier = PkceCodeVerifier::new(pkce_verifier_secret.to_string());

    let token_response = client
        .exchange_code(AuthorizationCode::new(code.to_string()))
        .map_err(|e| anyhow::anyhow!("exchange_code configuration error: {}", e))?
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http_client)
        .await
        .map_err(|e| anyhow::anyhow!("token exchange failed: {}", e))?;

    // Get the ID token and decode claims from JWT payload
    let id_token = token_response
        .id_token()
        .context("no ID token in response")?;

    let token_str = id_token.to_string();
    let parts: Vec<&str> = token_str.split('.').collect();
    let raw_claims: Value = if parts.len() >= 2 {
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[1])
            .unwrap_or_default();
        serde_json::from_slice(&decoded).unwrap_or(Value::Object(serde_json::Map::new()))
    } else {
        Value::Object(serde_json::Map::new())
    };

    let sub = raw_claims
        .get("sub")
        .and_then(|v| v.as_str())
        .context("missing 'sub' claim")?
        .to_string();

    let email = raw_claims
        .get("email")
        .and_then(|v| v.as_str())
        .map(String::from);
    let preferred_username = raw_claims
        .get("preferred_username")
        .and_then(|v| v.as_str())
        .map(String::from);
    let name = raw_claims
        .get("name")
        .and_then(|v| v.as_str())
        .map(String::from);

    let groups = raw_claims
        .get(role_claim_name)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(OidcClaims {
        sub,
        email,
        preferred_username,
        name,
        groups,
        raw: raw_claims,
    })
}

/// Test OIDC provider by performing discovery.
pub async fn test_discovery(provider: &SsoProviderRow, encryption_key: &[u8; 32]) -> Result<()> {
    let http_client = make_http_client()?;
    let (_client_id, _client_secret, issuer_url) = extract_oidc_config(provider, encryption_key)?;

    CoreProviderMetadata::discover_async(issuer_url, &http_client)
        .await
        .map_err(|e| anyhow::anyhow!("OIDC discovery failed: {}", e))?;

    Ok(())
}

fn extract_oidc_config(
    provider: &SsoProviderRow,
    encryption_key: &[u8; 32],
) -> Result<(ClientId, Option<ClientSecret>, IssuerUrl)> {
    let issuer_url = IssuerUrl::new(
        provider
            .oidc_issuer_url
            .as_deref()
            .context("OIDC issuer URL not configured")?
            .to_string(),
    )
    .context("invalid issuer URL")?;

    let client_id = ClientId::new(
        provider
            .oidc_client_id
            .as_deref()
            .context("OIDC client ID not configured")?
            .to_string(),
    );

    let client_secret = if let Some(encrypted) = &provider.oidc_client_secret_encrypted {
        let secret = crypto::decrypt(encrypted, encryption_key)
            .context("failed to decrypt client secret")?;
        Some(ClientSecret::new(secret))
    } else {
        None
    };

    Ok((client_id, client_secret, issuer_url))
}
