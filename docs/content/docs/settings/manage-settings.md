+++
title = "Settings"
description = "Configure your account, appearance, defaults, and platform-wide options"
weight = 91
date = 2025-03-06

[extra]
toc = true
+++

# Settings

The **Settings** page lets you configure your account profile, platform appearance, system defaults, logging, and license. Click **Settings** in the left sidebar to open it.

![Image: Settings page overview](/images/settings/settings-overview.png)

Settings are organized into seven tabs:

| Tab | What you can configure |
|-----|----------------------|
| **Account** | Profile, password, avatar |
| **Appearance** | Theme, timezone, language |
| **Logging** | Activity logs, export |
| **Defaults** | Default VM resource sizes |
| **System** | Platform info and database stats |
| **SSO** | Single Sign-On identity providers (OIDC) |
| **License** | Software license activation and status |

---

## Account Tab

![Image: Account tab](/images/settings/settings-account.png)

Manage your user profile:

- **Display Name / Avatar** — Upload a profile picture or change your display name
- **Change Password** — Enter your current password and a new one to update it
- **Profile Information** — Update username and other account details

### Changing Your Password

1. Go to **Settings → Account**
2. Scroll to the **Change Password** section
3. Enter your **current password**
4. Enter and confirm your **new password**
5. Click **Save**

---

## Appearance Tab

![Image: Appearance/Preferences tab](/images/settings/settings-preferences.png)

Customize the look and feel:

- **Theme** — Choose Dark, Light, or System default
- **Timezone** — Set your local timezone for timestamps
- **Date Format** — Switch between regional date formats
- **Language** — Interface language preference

Changes apply immediately without a page reload.

---

## Logging Tab

![Image: Logging/Audit tab](/images/settings/settings-logging.png)

View the platform activity log, which records system events such as:

- VM lifecycle operations (create, start, stop, delete)
- User logins and authentication events
- Container deployments
- License activations

Use the **Export** button to download logs as CSV for compliance or review purposes.

---

## Defaults Tab

![Image: Defaults tab](/images/settings/settings-defaults.png)

Configure default values pre-filled when creating new VMs:

- **vCPUs** — Default CPU count for new VMs
- **Memory** — Default RAM allocation (MB)
- **Boot Arguments** — Default kernel boot args
- **Image Selection** — Preferred kernel and rootfs images

These are user-level preferences — each user can set their own defaults.

---

## System Tab

![Image: System tab](/images/settings/settings-system.png)

View platform status and technical information:

- **Manager Version** — Current build version
- **Database** — PostgreSQL connection status and migration version
- **Uptime** — How long the Manager service has been running
- **Registered Hosts** — Count of active compute hosts  
- **API Endpoint** — Manager API URL for client configuration

This tab is read-only and useful for support and diagnostics.

---

## License Tab

![Image: License tab showing active license](/images/settings/settings-license.png)

Manage your software license:

### License Status

The License tab shows current activation status:

- 🟢 **Active** — License is valid and activated
- 🟡 **Grace Period** — License expired; limited time to re-activate
- 🔴 **Unlicensed** — No valid license; restricted to setup page

### Viewing License Details

When activated, you'll see:
- **Product** — Licensed product name
- **Customer** — Your organization name
- **License Key** — Masked key (e.g. `DGRG-****-****-T4BW`)
- **Expires** — License expiry date

### Activating a License

If your license is not yet activated:

1. Go to **Settings → License**
2. Click **Update License Key**
3. Enter your license key in `XXXX-XXXX-XXXX-XXXX` format
4. Click **Activate**

For offline activation, use the **Upload License File** option to upload a `.lic` file provided by Nexus Quantum.

### EULA

The License tab also shows your EULA acceptance status and links to the full End User License Agreement. Click **View EULA** to open the full agreement.

---

## SSO Tab

Configure **Single Sign-On** so users can authenticate against an external identity provider (OIDC) instead of a local username + password. Useful for organizations that already have an IdP like Keycloak, Okta, Auth0, or Azure AD.

### Adding a provider

1. Go to **Settings → SSO**.
2. Click **Add Provider**.
3. Fill in:
   - **Display name** — shown to users on the login page (e.g. `Corporate SSO`).
   - **Issuer URL** — the OIDC issuer (e.g. `https://idp.example.com/realms/main`).
   - **Client ID** / **Client secret** — credentials from your IdP application.
   - **Redirect URI** — copy this from the provider form and register it in your IdP.
   - **Scopes** — usually `openid email profile`.
4. Click **Save**.

The provider appears as a `Sign in with <name>` button on the login page.

### User provisioning

When a new user successfully authenticates via SSO for the first time, NQRust-MicroVM auto-creates a local user record bound to their IdP `sub` claim. Subsequent logins reuse the same record. You can manage these users from **Users** in the sidebar.

### Removing a provider

Open **Settings → SSO**, click **Remove** on the provider row. Existing SSO-authenticated users keep their accounts but can no longer log in through that provider until you re-add it.

---

## Platform Updates

A platform self-update mechanism (airgap `.nqupdate` bundles + internet manifest mode) is in development and not yet exposed in this UI build. Until it ships, update the platform by re-running the installer with the new release — see [Installation](../../getting-started/installation/).
