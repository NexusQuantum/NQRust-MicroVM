+++
title = "NQRust-MicroVM"
description = "Modern Firecracker microVM orchestration platform"
template = "index.html"
+++

# NQRust-MicroVM
### The Modern Firecracker Orchestrator

Manage **lightweight microVMs** and **Docker containers** with the safety of KVM, the speed of Rust, and the elegance of a modern React UI.

---

## Why NQRust-MicroVM?

**NQRust-MicroVM** is a high-performance management system for AWS Firecracker. It bridges the gap between raw KVM processes and a usable cloud platform. Designed for home labs and private clouds, it offers a slick dashboard to spin up VMs in milliseconds.

### Key Highlights

- **Instant VMs**: Boot Linux kernels in <125ms with minimal memory overhead
- **Container Hybrid**: Run Docker containers inside isolated Firecracker VMs for maximum security
- **Web Terminal**: Full browser-based shell access via WebSocket - no SSH client needed
- **Serverless Functions**: Deploy Node.js, Python, and Ruby functions with cold starts under 10ms
- **Bridged Networking**: VMs appear as physical devices on your LAN with DHCP support
- **Snapshots**: Capture exact VM states and restore them instantly
- **Modern UI**: Next.js 15 + React 19 frontend with real-time metrics and monitoring
- **Production Ready**: PostgreSQL backend, RBAC, audit logs, and multi-host orchestration

---

## Quick Start

Get up and running in minutes with our comprehensive guides:

- [**Installation Guide**](/getting-started/installation/) - Step-by-step setup instructions
- [**Quick Start**](/getting-started/quick-start/) - Create your first VM in 5 minutes
- [**User Guide**](/user-guide/) - Complete feature documentation

---

## Architecture

The system is composed of three lightweight components communicating over HTTP and WebSockets:

- **Manager** (Rust/Axum): Central orchestration API managing VM lifecycle, containers, and functions
- **Agent** (Rust): Host agent executing VM operations via Firecracker
- **UI** (Next.js 15): Modern React frontend with real-time metrics
- **Guest Agent** (Rust): In-VM metrics reporting

See the [Introduction](/introduction/) for detailed architecture overview.

---

## Features

Explore what NQRust-MicroVM can do:

- [**VM Management**](/user-guide/vm-management/) - Create, manage, and monitor microVMs
- [**Containers**](/user-guide/containers/) - Docker container orchestration in isolated VMs
- [**Serverless Functions**](/user-guide/functions/) - Execute Node.js, Python, and Ruby functions
- [**Networking**](/user-guide/networking/) - Advanced networking with VLAN support
- [**Storage**](/user-guide/storage/) - Volume management and image registry
- [**Infrastructure**](/user-guide/infrastructure/) - Multi-host management and monitoring

---

## Getting Help

- [Documentation](/getting-started/) - Comprehensive guides and tutorials
- [GitHub Issues](https://github.com/yourusername/nqrust-microvm/issues) - Report bugs or request features
- [API Reference](http://localhost:18080/swagger-ui/) - OpenAPI documentation

---

## License

Distributed under the **GNU Affero General Public License v3.0**. See LICENSE for more information.

Built with ❤️ by the Nexus Team. Powered by Rust & Caffeine.
