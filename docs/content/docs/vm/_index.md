+++
title = "Virtual Machines"
description = "Create and manage lightweight Firecracker microVMs"
weight = 30
date = 2025-12-16
+++

Virtual Machines in NQRust-MicroVM are powered by Firecracker, providing lightweight, secure, and fast virtualization for your workloads.

---

## Overview

NQRust-MicroVM uses Firecracker to create microVMs - minimal virtual machines designed for serverless and container workloads. Each VM provides complete isolation with dedicated resources.

**[IMAGE: vm-overview.png - Screenshot of VMs dashboard showing list of running VMs]**

#### Key Features

- **Lightning Fast Boot** - VMs start in under 125ms
- **Minimal Overhead** - Only 5 MB memory per VM
- **Strong Isolation** - Hardware-virtualized security
- **Full Linux Support** - Run Ubuntu, Alpine, Debian, and more
- **Web Console** - Browser-based terminal access
- **Live Metrics** - Real-time CPU, memory, and network monitoring

---

## Quick Start

**Create your first VM in 3 minutes**:

1. Navigate to **Virtual Machines** in the sidebar
2. Click **Create VM** button
3. Follow the 5-step wizard
4. Access your VM via web console

---

## VM Lifecycle

**[IMAGE: vm-lifecycle.png - Diagram showing VM states: Stopped → Running → Paused]**

VMs can be in the following states:

- **Stopped** - VM is created but not running
- **Running** - VM is active and consuming resources
- **Paused** - VM is frozen, can be resumed quickly
- **Failed** - VM encountered an error

---

## Common Use Cases

#### Development Environments
Create isolated dev environments for each team member with consistent configurations.

**[IMAGE: usecase-dev.png - Screenshot showing multiple dev VMs]**

#### Testing & CI/CD
Spin up fresh test environments for each test run, then destroy them automatically.

#### Production Workloads
Run microservices with strong isolation and minimal overhead.

---

## Getting Started

Choose a topic to learn more:

- **[Create a VM](create-vm/)** - Step-by-step VM creation guide
- **[Access VM](access-vm/)** - Connect to your VM via console or SSH
- **[Manage VMs](manage-vm/)** - Start, stop, pause, resume operations
- **[Backup & Snapshot](backup-snapshot/)** - Protect your VM data
- **[Monitoring](monitoring/)** - View real-time metrics and logs
