use crate::features::sso::repo::{SsoProviderRow, SsoRepoError, UserIdentityRepository};
use crate::features::users::repo::UserRepository;
use anyhow::Result;
use nexus_types::Role;
use tracing::{debug, info};

/// Resolve an internal role based on IdP groups and the provider's role mapping config.
pub fn resolve_role(provider: &SsoProviderRow, groups: &[String]) -> Role {
    if let Some(mapping) = &provider.role_mapping {
        if let Some(obj) = mapping.as_object() {
            // Check in priority order: admin > user > viewer
            for (role_str, expected_groups) in [
                ("admin", Role::Admin),
                ("user", Role::User),
                ("viewer", Role::Viewer),
            ] {
                if let Some(allowed) = obj.get(role_str).and_then(|v| v.as_array()) {
                    for group in groups {
                        if allowed.iter().any(|g| g.as_str() == Some(group.as_str())) {
                            debug!(role = role_str, group = %group, "role mapped from IdP group");
                            return expected_groups;
                        }
                    }
                }
            }
        }
    }

    // Fall back to provider default
    provider.default_role.parse().unwrap_or(Role::Viewer)
}

/// Derive a username from SSO claims, trying multiple fallbacks.
pub fn derive_username(
    preferred_username: Option<&str>,
    email: Option<&str>,
    external_id: &str,
) -> String {
    if let Some(username) = preferred_username {
        if !username.is_empty() {
            return username.to_string();
        }
    }
    if let Some(email) = email {
        if let Some(prefix) = email.split('@').next() {
            if !prefix.is_empty() {
                return prefix.to_string();
            }
        }
    }
    external_id.to_string()
}

/// Result of SSO user provisioning.
pub struct ProvisionResult {
    pub user: crate::features::users::repo::UserRow,
    pub was_created: bool,
}

/// Find an existing user linked to this SSO identity, or create one via JIT provisioning.
#[allow(clippy::too_many_arguments)]
pub async fn find_or_provision_user(
    user_repo: &UserRepository,
    identity_repo: &UserIdentityRepository,
    provider: &SsoProviderRow,
    external_id: &str,
    email: Option<&str>,
    preferred_username: Option<&str>,
    display_name: Option<&str>,
    groups: &[String],
    raw_claims: Option<serde_json::Value>,
) -> Result<ProvisionResult, SsoProvisionError> {
    // 1. Check if identity link already exists
    if let Ok(identity) = identity_repo
        .find_by_external_id(provider.id, external_id)
        .await
    {
        let user = user_repo
            .get_by_id(identity.user_id)
            .await
            .map_err(|_| SsoProvisionError::UserLookupFailed)?;
        identity_repo.update_last_login(identity.id).await.ok(); // non-fatal
        info!(user_id = %user.id, external_id = %external_id, "existing SSO user logged in");
        return Ok(ProvisionResult {
            user,
            was_created: false,
        });
    }

    // 2. JIT provisioning check
    if !provider.allow_jit_provisioning {
        return Err(SsoProvisionError::ProvisioningDisabled);
    }

    // 3. Derive username
    let username = derive_username(preferred_username, email, external_id);

    // 4. Check for existing local user with same username → link accounts
    if let Ok(existing) = user_repo.get_by_username(&username).await {
        identity_repo
            .create_link(
                existing.id,
                provider.id,
                external_id,
                email,
                display_name,
                raw_claims,
            )
            .await
            .map_err(|_| SsoProvisionError::LinkCreationFailed)?;

        // Update auth_source to "both"
        user_repo.set_auth_source(existing.id, "both").await.ok(); // non-fatal

        info!(user_id = %existing.id, username = %username, "linked SSO identity to existing user");
        return Ok(ProvisionResult {
            user: existing,
            was_created: false,
        });
    }

    // 5. Create new SSO user
    let role = resolve_role(provider, groups);
    let user = user_repo
        .create_sso_user(&username, email, role)
        .await
        .map_err(|_| SsoProvisionError::UserCreationFailed)?;

    // 6. Create identity link
    identity_repo
        .create_link(
            user.id,
            provider.id,
            external_id,
            email,
            display_name,
            raw_claims,
        )
        .await
        .map_err(|_| SsoProvisionError::LinkCreationFailed)?;

    info!(user_id = %user.id, username = %username, role = %role, "JIT-provisioned new SSO user");
    Ok(ProvisionResult {
        user,
        was_created: true,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum SsoProvisionError {
    #[error("JIT provisioning is disabled for this provider")]
    ProvisioningDisabled,
    #[error("failed to look up user")]
    UserLookupFailed,
    #[error("failed to create user")]
    UserCreationFailed,
    #[error("failed to create identity link")]
    LinkCreationFailed,
    #[error(transparent)]
    Repo(#[from] SsoRepoError),
}
