+++
title = "API Reference"
description = "Interactive REST API reference for NQRust-MicroVM — browse all endpoints, schemas, and try requests live"
icon = "code"
weight = 95
layout = "single"
toc = true
+++

NQRust-MicroVM exposes a full REST API served by the **Manager** service. The interactive API reference is bundled directly into the web UI — no external tools needed.

---

## Accessing the API Reference

Open the following URL in your browser, replacing `<microvm-ip>` with your server's IP address or hostname:

```
http://<microvm-ip>:3000/docs
```

{{% alert icon="💡" context="info" %}}
`3000` is the default port for the NQRust-MicroVM web UI. If you configured a different port during installation, use that port instead.
{{% /alert %}}

The API reference is built into the UI and automatically uses the correct base URL for your host — no manual configuration required.

---

## What's Included

The API reference covers all available endpoints, organized by resource:

| Section | Description |
|---|---|
| **Auth** | Login, token management |
| **VMs** | Create, start, stop, delete, list virtual machines |
| **VM Configuration** | CPU, memory, networking, boot settings |
| **VM Devices** | Drives, network interfaces |
| **Containers** | Deploy and manage Docker containers inside VMs |
| **Functions** | Serverless function lifecycle |
| **Images** | Image registry — import, browse, manage |
| **Snapshots** | VM state capture and restore |
| **Templates** | Reusable VM configuration templates |
| **Hosts** | Host agent registration and management |
| **Users** | User accounts and RBAC |
| **Logs** | Container and function log streaming |

---

## Authentication

All API calls (except login) require a Bearer token:

```bash
# 1. Get a token
curl -X POST http://<microvm-ip>:18080/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "root", "password": "root"}'

# 2. Use the token
curl -H "Authorization: Bearer <your-token>" \
  http://<microvm-ip>:18080/v1/vms
```

The **Base URL** for all API calls is:

```
http://<microvm-ip>:18080/v1
```

{{% alert icon="⚠️" context="warning" %}}
The API reference UI at `:3000/docs` automatically constructs the correct base URL from your current browser hostname. The API itself runs on port **18080** (Manager service), not 3000.
{{% /alert %}}

---

## OpenAPI Spec

The raw OpenAPI 3.0 spec is available at:

```
http://<microvm-ip>:18080/api-docs/openapi.json
```

Use this to generate client SDKs with tools like `openapi-generator` or import into Postman / Insomnia.
