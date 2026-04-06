use crate::features::sso::repo::SsoProviderRow;
use anyhow::{Context, Result};
use base64::Engine;
use std::collections::HashMap;

/// Claims extracted from a successful SAML assertion.
#[derive(Debug, Clone)]
pub struct SamlClaims {
    pub name_id: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub groups: Vec<String>,
    pub attributes: HashMap<String, Vec<String>>,
}

/// Generate SP metadata XML for the given provider.
pub fn generate_sp_metadata(provider: &SsoProviderRow, base_url: &str) -> Result<String> {
    let acs_url = format!("{}/v1/sso/saml/{}/acs", base_url, provider.slug);
    let entity_id = provider
        .saml_sp_entity_id
        .as_deref()
        .map(String::from)
        .unwrap_or_else(|| format!("{}/v1/sso/saml/{}/metadata", base_url, provider.slug));

    let name_id_format = provider
        .saml_name_id_format
        .as_deref()
        .unwrap_or("urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress");

    // Manually construct SP metadata XML
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<md:EntityDescriptor xmlns:md="urn:oasis:names:tc:SAML:2.0:metadata" entityID="{entity_id}">
  <md:SPSSODescriptor AuthnRequestsSigned="false" WantAssertionsSigned="true"
      protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">
    <md:NameIDFormat>{name_id_format}</md:NameIDFormat>
    <md:AssertionConsumerService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"
        Location="{acs_url}" index="0" isDefault="true"/>
  </md:SPSSODescriptor>
</md:EntityDescriptor>"#
    );

    Ok(xml)
}

/// Create a SAML AuthnRequest and return the redirect URL to the IdP.
pub fn create_authn_request(provider: &SsoProviderRow, base_url: &str) -> Result<String> {
    let idp_sso_url = provider
        .saml_idp_sso_url
        .as_deref()
        .context("SAML IdP SSO URL not configured")?;

    let entity_id = provider
        .saml_sp_entity_id
        .as_deref()
        .map(String::from)
        .unwrap_or_else(|| format!("{}/v1/sso/saml/{}/metadata", base_url, provider.slug));

    let acs_url = format!("{}/v1/sso/saml/{}/acs", base_url, provider.slug);
    let request_id = format!("_{}", uuid::Uuid::new_v4());
    let issue_instant = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");

    let authn_request = format!(
        r#"<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol"
    ID="{request_id}"
    Version="2.0"
    IssueInstant="{issue_instant}"
    Destination="{idp_sso_url}"
    AssertionConsumerServiceURL="{acs_url}"
    ProtocolBinding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST">
  <saml:Issuer xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion">{entity_id}</saml:Issuer>
</samlp:AuthnRequest>"#
    );

    let encoded = base64::engine::general_purpose::STANDARD.encode(authn_request.as_bytes());
    let redirect_url = format!(
        "{}?SAMLRequest={}",
        idp_sso_url,
        urlencoding::encode(&encoded)
    );

    Ok(redirect_url)
}

/// Parse a SAML Response from the ACS endpoint.
/// Note: This implementation does NOT validate XML signatures.
/// For production use, install xmlsec1 and enable the `xmlsec` feature on `samael`.
pub fn process_response(
    _provider: &SsoProviderRow,
    saml_response_b64: &str,
    _base_url: &str,
    role_claim_name: &str,
) -> Result<SamlClaims> {
    let response_bytes = base64::engine::general_purpose::STANDARD
        .decode(saml_response_b64)
        .context("failed to decode SAML response")?;

    let response_str =
        String::from_utf8(response_bytes).context("SAML response is not valid UTF-8")?;

    // Parse the SAML response using samael's schema types
    let response: samael::schema::Response = samael::metadata::de::from_str(&response_str)
        .context("failed to parse SAML response XML")?;

    // Check status
    if let Some(status) = &response.status {
        let code = &status.status_code;
        let success = "urn:oasis:names:tc:SAML:2.0:status:Success";
        if code.value.as_deref() != Some(success) {
            anyhow::bail!(
                "SAML response status: {}",
                code.value.as_deref().unwrap_or("unknown")
            );
        }
    }

    // Get assertion
    let assertion = response
        .assertion
        .as_ref()
        .context("no assertion in SAML response")?;

    // Extract NameID
    let name_id = assertion
        .subject
        .as_ref()
        .and_then(|s| s.name_id.as_ref())
        .map(|n| n.value.clone())
        .context("missing NameID in assertion")?;

    // Extract attributes
    let mut attributes: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(attr_statements) = &assertion.attribute_statements {
        for stmt in attr_statements {
            for attr in &stmt.attributes {
                let name = attr.name.as_deref().unwrap_or("").to_string();
                let values: Vec<String> =
                    attr.values.iter().filter_map(|v| v.value.clone()).collect();
                attributes.insert(name, values);
            }
        }
    }

    let email = attributes
        .get("email")
        .or_else(|| {
            attributes.get("http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress")
        })
        .and_then(|v| v.first().cloned());

    let display_name = attributes
        .get("displayName")
        .or_else(|| attributes.get("http://schemas.xmlsoap.org/ws/2005/05/identity/claims/name"))
        .and_then(|v| v.first().cloned());

    let groups = attributes
        .get(role_claim_name)
        .or_else(|| attributes.get("http://schemas.xmlsoap.org/claims/Group"))
        .cloned()
        .unwrap_or_default();

    Ok(SamlClaims {
        name_id,
        email,
        display_name,
        groups,
        attributes,
    })
}
