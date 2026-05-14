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
| **License** | Software license activation and status |
| **Updates** | Platform self-update (airgap bundles or internet manifest) — added in v0.3.0 |

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

## Updates Tab

Added in **v0.3.0** — lets admins apply new NQRust-MicroVM releases without leaving the dashboard. Two delivery modes:

### Airgap mode (upload `.nqupdate` bundle)

1. Download a release bundle from your release distribution channel (file is `nqrust-vX.Y.Z.nqupdate`).
2. Go to **Settings → Updates**.
3. Drag the bundle into the upload area or click **Choose file** and pick it.
4. The page shows the bundle's version + checksum once parsed.
5. Click **Apply update** and confirm.

The platform applies the update in order: **Manager → Agents (rolling) → UI**. Running VMs are **not** disturbed by agent restart — the agent re-attaches to live Firecracker sockets on startup.

If the new manager fails its post-update health check, the auto-updater rolls the manager binary back to the prior version automatically. Database migrations are **not** rolled back (they're forward-only per release policy — see the developer docs).

### Internet mode (manifest URL)

If your platform has outbound network:

1. Toggle **Enable internet update checks**.
2. Provide a manifest URL (e.g. `https://updates.example.com/manifest.json`).
3. The manager polls the manifest periodically; when a newer version is available, the page shows it under **Available updates**.
4. Click **Apply** to start the rolling upgrade.

### Notes

- Both modes go through the same apply pipeline; only the *delivery* differs.
- The `.nqupdate` bundle bundles the manager, agent, guest-agent, UI, and any new migrations.
- Each component binary is installed under `/opt/nqrust/bin/<name>.<version>` with a `<name>` symlink — old versions are kept around to enable rollback.
- systemd units use `RestartForceExitStatus=42` so a clean self-update exit triggers a restart on the new binary.
