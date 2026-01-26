# Docs Deployment Guide

This guide explains how to set up automatic deployment of the Hugo documentation to your Biznet Gio VM.

## Overview

The deployment uses:
- **GitHub Actions** - Builds Hugo docs and deploys via SSH
- **Caddy** - Serves static files with automatic HTTPS (if using a domain)
- **rsync** - Efficient file synchronization

## Prerequisites

- A Biznet Gio VM with Ubuntu/Debian or RHEL-based OS
- SSH access to the VM
- GitHub repository with Actions enabled

## Step 1: Set Up the VM

SSH into your Biznet Gio VM and run the setup script:

```bash
# Download and run the setup script
curl -O https://raw.githubusercontent.com/YOUR_ORG/NQRust-MicroVM/main/scripts/setup-docs-server.sh
chmod +x setup-docs-server.sh

# For IP-only access (HTTP)
sudo ./setup-docs-server.sh 103.xxx.xxx.xxx

# For domain with automatic HTTPS
sudo ./setup-docs-server.sh docs.yourdomain.com
```

This script will:
1. Install Caddy web server
2. Create a `deploy` user for GitHub Actions
3. Set up the `/var/www/docs` directory
4. Configure Caddy to serve the docs

## Step 2: Generate SSH Keys

On your VM (or local machine), generate an SSH key pair for deployments:

```bash
# Generate a new key pair
ssh-keygen -t ed25519 -f github_deploy_key -N '' -C "github-actions-deploy"

# Add public key to deploy user
sudo cat github_deploy_key.pub >> /home/deploy/.ssh/authorized_keys

# Display the private key (you'll need this for GitHub)
cat github_deploy_key
```

## Step 3: Configure GitHub Secrets

Go to your GitHub repository → **Settings** → **Secrets and variables** → **Actions**

Add these secrets:

| Secret Name | Value | Example |
|------------|-------|---------|
| `VM_HOST` | Your VM's IP address | `103.123.45.67` |
| `VM_USER` | Deploy username | `deploy` |
| `VM_SSH_KEY` | Contents of `github_deploy_key` (private key) | `-----BEGIN OPENSSH PRIVATE KEY-----...` |
| `VM_SSH_PORT` | SSH port (optional, defaults to 22) | `22` |
| `DOCS_BASE_URL` | Base URL for the docs site | `http://103.123.45.67/` or `https://docs.example.com/` |

### Setting the SSH Key Secret

1. Copy the **entire** contents of your private key file:
   ```bash
   cat github_deploy_key
   ```

2. In GitHub, create the `VM_SSH_KEY` secret and paste the key, including:
   - `-----BEGIN OPENSSH PRIVATE KEY-----`
   - All the content
   - `-----END OPENSSH PRIVATE KEY-----`

## Step 4: Test the Deployment

1. Make a small change to any file in the `docs/` directory
2. Commit and push to the `main` branch
3. Go to **Actions** tab in GitHub to watch the deployment
4. Once complete, visit your VM's IP address to see the docs

### Manual Trigger

You can also trigger the deployment manually:
1. Go to **Actions** → **Deploy Docs to VM**
2. Click **Run workflow** → **Run workflow**

## Workflow File

The deployment workflow is at [`.github/workflows/deploy-docs-vm.yml`](../.github/workflows/deploy-docs-vm.yml).

It triggers on:
- Push to `main` branch (changes in `docs/` folder)
- Manual trigger (workflow_dispatch)

## Troubleshooting

### SSH Connection Failed

1. Verify the VM_HOST is correct
2. Check the SSH key was added to authorized_keys:
   ```bash
   cat /home/deploy/.ssh/authorized_keys
   ```
3. Ensure SSH port is open in firewall:
   ```bash
   sudo ufw allow 22/tcp
   ```

### Permission Denied on Deploy

Ensure the deploy user owns the docs directory:
```bash
sudo chown -R deploy:deploy /var/www/docs
```

### Caddy Not Serving Files

Check Caddy status:
```bash
sudo systemctl status caddy
sudo journalctl -u caddy -f
```

Validate Caddyfile:
```bash
sudo caddy validate --config /etc/caddy/Caddyfile
```

### View Deployment Logs

In GitHub, go to **Actions** → select the workflow run → click on the job to see logs.

## Using a Custom Domain

To use a domain (e.g., `docs.yourdomain.com`):

1. Point your domain's DNS to your VM's IP:
   - A Record: `docs` → `103.xxx.xxx.xxx`

2. Update the Caddyfile on your VM:
   ```bash
   sudo nano /etc/caddy/Caddyfile
   ```
   Change the first line from `:80` to your domain:
   ```
   docs.yourdomain.com {
       ...
   }
   ```

3. Restart Caddy (it will automatically get an SSL certificate):
   ```bash
   sudo systemctl restart caddy
   ```

4. Update `DOCS_BASE_URL` secret in GitHub to `https://docs.yourdomain.com/`

## Local Development

To preview docs locally:

```bash
cd docs
hugo server -D
```

Open http://localhost:1313 in your browser.

## Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   GitHub Repo   │────▶│  GitHub Actions │────▶│  Biznet Gio VM  │
│                 │     │                 │     │                 │
│  docs/          │     │  1. Build Hugo  │     │  /var/www/docs  │
│  ├── content/   │     │  2. rsync SSH   │     │  └── index.html │
│  └── hugo.toml  │     │                 │     │                 │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                                                       │
                                                       ▼
                                                ┌─────────────────┐
                                                │     Caddy       │
                                                │  (Web Server)   │
                                                └─────────────────┘
                                                       │
                                                       ▼
                                                   Users
```
