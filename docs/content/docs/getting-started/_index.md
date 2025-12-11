+++
title = "Getting Started"
description = "Get up and running with NQRust-MicroVM"
weight = 2
sort_by = "weight"
template = "section.html"
page_template = "page.html"
+++

# Getting Started

Welcome to NQRust-MicroVM! This section will guide you through installing the platform and creating your first microVM.

+++

## Overview

NQRust-MicroVM is a modern platform for managing AWS Firecracker microVMs with a focus on ease of use, security, and performance. Whether you're building a home lab, private cloud, or edge infrastructure, this guide will help you get started quickly.

+++

## What You'll Learn

This section covers everything you need to begin using NQRust-MicroVM:

### [Installation Guide](/getting-started/installation/)
Complete step-by-step installation instructions including:
- System requirements and prerequisites
- Installing dependencies (Rust, Node.js, PostgreSQL, Firecracker)
- Building the project from source
- Configuring networking and storage
- Starting all services
- Troubleshooting common issues

**Time required:** 30-45 minutes

### [Quick Start](/getting-started/quick-start/)
Create your first microVM in minutes:
- Starting the services
- Getting VM images (kernel and rootfs)
- Creating your first VM via Web UI or API
- Accessing the VM terminal
- Basic VM lifecycle operations (start, stop, pause, resume)
- Creating snapshots and templates

**Time required:** 5-10 minutes (after installation)

+++

## Prerequisites

Before you begin, ensure you have:

**Hardware:**
- x86_64 CPU with KVM support (Intel VT-x or AMD-V)
- 4GB+ RAM (8GB recommended)
- 20GB+ free disk space

**Software:**
- Linux (Ubuntu 22.04+ recommended)
- Root/sudo access for KVM and network configuration
- Internet connection for downloading dependencies

**Skills:**
- Basic Linux command-line knowledge
- Familiarity with terminal/shell operations
- (Optional) Understanding of virtualization concepts

+++

## Installation Options

### Full Development Environment
Follow the complete [Installation Guide](/getting-started/installation/) to set up a development environment with all services running locally. This is ideal for:
- Development and testing
- Learning how the platform works
- Contributing to the project
- Home lab deployments

### Quick Demo (Coming Soon)
Looking for a faster way to try NQRust-MicroVM? We're working on:
- Docker Compose setup for quick demos
- Pre-built binaries for common platforms
- Installer script for automated setup

+++

## After Installation

Once you've completed the installation and quick start guides:

1. **Explore Features**
   - [VM Management](/user-guide/vm-management/) - Advanced VM operations
   - [Containers](/user-guide/containers/) - Docker container orchestration
   - [Serverless Functions](/user-guide/functions/) - Deploy Node.js, Python, Ruby functions
   - [Networking](/user-guide/networking/) - Advanced networking with VLANs
   - [Storage](/user-guide/storage/) - Volume and image management

2. **Production Deployment**
   - [Performance Tuning](/operations/performance-tuning/) - Optimize for production workloads
   - [Bridged Networking](/operations/bridged-networking/) - Connect VMs to physical network
   - [Deployment Guide](/deployment/) - Production deployment strategies

3. **Development**
   - [API Reference](http://localhost:18080/swagger-ui/) - Complete REST API documentation
   - [Architecture](/introduction/) - Understanding system components
   - [Contributing](/development/contributing/) - Contribute to the project

+++

## Getting Help

If you encounter issues during setup:

1. **Check Troubleshooting Section** - The [Installation Guide](/getting-started/installation/#troubleshooting) has solutions for common problems
2. **Review Logs** - Check terminal output for error messages
3. **Verify Prerequisites** - Ensure KVM support, permissions, and dependencies are correct
4. **GitHub Issues** - Report bugs or ask questions on GitHub
5. **Documentation** - Search this documentation site for answers

+++

## System Architecture

Understanding the components helps during installation:

```
┌─────────────────────────────────────────────────────┐
│                  User / Browser                      │
└────────────┬────────────────────────────────────────┘
             │ HTTPS/WebSocket
             ↓
┌────────────────────────────────────────────────────┐
│              Next.js Frontend (Port 3000)           │
│              - React 19 + TypeScript                │
│              - Real-time metrics & terminal         │
└────────────┬───────────────────────────────────────┘
             │ REST API
             ↓
┌────────────────────────────────────────────────────┐
│           Rust Manager API (Port 18080)             │
│           - VM lifecycle orchestration              │
│           - PostgreSQL for state                    │
│           - WebSocket support                       │
└────────────┬───────────────────────────────────────┘
             │ REST API
             ↓
┌────────────────────────────────────────────────────┐
│          Rust Host Agent (Port 9090)                │
│          - Firecracker VM management                │
│          - Requires root/KVM access                 │
└────────────┬───────────────────────────────────────┘
             │ Unix Socket
             ↓
┌────────────────────────────────────────────────────┐
│                Firecracker MicroVMs                 │
│                - Boot in <125ms                     │
│                - Guest agent for metrics            │
└────────────────────────────────────────────────────┘
```

**Key Points:**
- **Manager** is the central API that orchestrates everything
- **Agent** runs on KVM hosts and talks to Firecracker
- **Frontend** provides the web UI
- **Guest Agent** runs inside VMs for metrics reporting

See [Introduction](/introduction/) for detailed architecture overview.

+++

## Next Steps

Ready to begin? Start with the [Installation Guide](/getting-started/installation/)!

Already installed? Jump to the [Quick Start Guide](/getting-started/quick-start/) to create your first VM.
