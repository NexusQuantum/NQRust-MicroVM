# NQRust-MicroVM - Complete Feature Documentation

**Version**: 0.1.0  
**Last Updated**: October 2025  
**Target Audience**: UI/UX Design Team

---

## Table of Contents

1. [Executive Overview](#executive-overview)
2. [Frontend Architecture](#frontend-architecture)
3. [Frontend Pages & Routes](#frontend-pages--routes)
4. [Core Features & API Reference](#core-features--api-reference)
5. [Data Models](#data-models)
6. [Implementation Status](#implementation-status)
7. [Edge Cases & Error Handling](#edge-cases--error-handling)

---

## Executive Overview

NQRust-MicroVM is a control plane system for managing lightweight virtual machines (microVMs) powered by Firecracker. It provides a unified REST API and web UI for:

- **Multi-VM Lifecycle Management**: Create, start, stop, pause, resume, and delete VMs
- **Persistent Storage**: Add, update, and manage drives per VM
- **Network Configuration**: Create and manage network interfaces with rate limiting
- **Snapshots**: Create, restore, and manage VM snapshots for fast provisioning
- **Templates**: Define golden VM configurations for quick replication
- **Image Registry**: Centralized kernel and rootfs image management
- **Monitoring**: Log tailing and metrics collection

### Architecture

```
Frontend (Next.js)
    ↓
API Proxy (/api/proxy/v1)
    ↓
Manager (Rust/Axum + PostgreSQL)
    ↓
Agent (Rust/Axum - runs on each host)
    ↓
Firecracker + Host OS
```

---

## Frontend Architecture

### Technology Stack

- **Framework**: Next.js 14+ (App Router)
- **Language**: TypeScript
- **State Management**: TanStack React Query v5
- **HTTP Client**: Axios-based `apiClient` with error handling
- **UI Components**: shadcn/ui + Lucide icons
- **WebSocket**: Real-time terminal access

### API Communication

The frontend communicates with the backend through:

1. **REST API**: `/api/proxy/v1/*` → forwarded to Manager API
2. **WebSocket**: Real-time VM terminal access at `ws://localhost:18081/v1/vms/{id}/shell/ws`
3. **React Query**: Automatic caching, refetching, and state synchronization

### Base URL Configuration

```typescript
// Production
NEXT_PUBLIC_API_BASE_URL=http://localhost:18080/v1
NEXT_PUBLIC_WS_BASE_URL=ws://localhost:18080

// Development (via proxy)
NEXT_PUBLIC_API_BASE_URL=http://localhost:3000/api/proxy/v1
NEXT_PUBLIC_WS_BASE_URL=ws://localhost:3000
```

---

## Frontend Pages & Routes

### 1. Home Page (`/`)

**Purpose**: Landing page or redirect to dashboard

**Status**: Basic implementation

**User Interactions**: Typically redirects to `/dashboard`

### 2. Health Check (`/health`)

**Purpose**: System health monitoring endpoint

**Status**: Basic implementation

**Response**: Health status of the system

**Used By**: Load balancers, monitoring systems

---

### 3. Dashboard (`/dashboard`)

**Purpose**: Overview of all VMs and system status

**Status**: Implemented with VMs list view

**Key Components**:
- VM list with status indicators
- Quick action buttons
- Search/filter VMs by name or state
- Navigation to VM details or creation

**User Interactions**:
- View all running/stopped VMs
- Launch VM creation wizard
- Access individual VM details
- Perform bulk VM operations (future)

**Data Fetched**:
- `useVMs()` - Lists all VMs with current state
- Auto-refresh every 30 seconds

---

### 4. VMs List (`/vms`)

**Purpose**: Detailed list of all virtual machines

**Status**: Fully implemented

**Page Structure**:

```
┌─ Header ─────────────────────────────────┐
│ VMs                      [+ Create VM]   │
├────────────────────────────────────────── ┤
│ Name     │ State  │ vCPU │ Memory │ Host │
├──────────┼────────┼──────┼────────┼──────┤
│ vm-prod1 │ Running│  2   │ 512 MB │ host1│
│ vm-test  │ Stopped│  1   │ 256 MB │ host2│
└────────────────────────────────────────── ┘
```

**Features**:
- Real-time VM state display
- Inline quick actions (Start, Stop, Delete)
- Column sorting and filtering
- Search by name

**Key Data Fields**:
- `name`: VM identifier
- `state`: Current state (creating, running, paused, stopped, deleted)
- `vcpu`: Number of vCPUs
- `mem_mib`: Memory in MiB
- `host_id`: Which host is running the VM
- `created_at`, `updated_at`: Timestamps

**React Query Hooks Used**:
- `useVMs()` - Fetch all VMs
- `useVmStatePatch()` - Perform actions

---

### 5. VM Details (`/vms/[id]`)

**Purpose**: Detailed view and management of a single VM

**Status**: Fully implemented with multiple tabs

**Page Structure**:

```
┌─ VM Name  [Status Badge]     [Action Menu] ┐
├──────────────────────────────────────────── ┤
│ Overview │ Config │ Drives │ Network │ ... │
├──────────────────────────────────────────── ┤
│ [Tab Content]                               │
└──────────────────────────────────────────── ┘
```

**Tabs**:

#### Tab 1: Overview
- VM metadata (ID, name, created date)
- State and host information
- Quick status panel

#### Tab 2: Configuration
- vCPU count
- Memory allocation
- Kernel path
- Rootfs path
- Template info

#### Tab 3: Drives
- List of attached drives
- Drive operations (Add, Edit, Remove)
- Root device indicator
- Read-only status

#### Tab 4: Network
- Network interfaces (NICs)
- MAC addresses
- Host device mappings
- Rate limiting settings

#### Tab 5: Snapshots
- List of snapshots for this VM
- Create snapshot button
- Restore snapshot actions
- Delete snapshot options

#### Tab 6: Metrics (Future)
- CPU usage graphs
- Memory utilization
- Network I/O graphs
- Metrics export

#### Tab 7: Terminal (Future)
- WebSocket-based terminal access
- Real-time shell interaction

**Action Menu**:
- Start VM
- Stop VM
- Pause VM
- Resume VM
- Delete VM
- Create Snapshot
- Flush Metrics
- Send Ctrl+Alt+Del

**React Query Hooks Used**:
- `useVM(id)` - Fetch VM details
- `useVmStatePatch()` - Execute actions
- `useSnapshots()` - List snapshots
- `useVMDrives()` - List drives
- `useVMNics()` - List network interfaces

---

### 6. Create VM (`/vms/create`)

**Purpose**: Wizard for creating new VMs

**Status**: Fully implemented

**Wizard Steps**:

#### Step 1: Basic Configuration
- **VM Name** (required, text input)
- **vCPU Count** (required, number input, min: 1)
- **Memory (MiB)** (required, number input, min: 256)

```typescript
// Example input
{
  name: "my-vm-prod",
  vcpu: 2,
  mem_mib: 512
}
```

#### Step 2: Boot Configuration
- **Kernel Source** (toggle between image ID or raw path)
  - If using image ID: dropdown of kernel images from registry
  - If using path: text input for kernel path
- **Rootfs Source** (same options as kernel)

```typescript
// Example with image IDs
{
  kernel_image_id: "59e1c754-2210-4887-858c-f3c5de7d483b",
  rootfs_image_id: "4196a86f-95f4-4609-af23-138ec331b0dc"
}

// Alternative with paths (if MANAGER_ALLOW_IMAGE_PATHS=1)
{
  kernel_path: "/srv/images/kernel.bin",
  rootfs_path: "/srv/images/alpine-rootfs.ext4"
}
```

#### Step 3: Advanced Options (Optional)
- **Network Bridge** (future: auto-select available bridges)
- **Additional Drives** (future)
- **Network Interfaces** (future)

#### Step 4: Review & Create
- Summary of configuration
- Create button

**API Call on Completion**:

```http
POST /v1/vms
Content-Type: application/json

{
  "name": "my-vm-prod",
  "vcpu": 2,
  "mem_mib": 512,
  "kernel_image_id": "59e1c754-2210-4887-858c-f3c5de7d483b",
  "rootfs_image_id": "4196a86f-95f4-4609-af23-138ec331b0dc"
}
```

**React Query Hooks Used**:
- `useRegistryImages()` - Populate kernel/rootfs options
- `useCreateVM()` - Submit creation request

---

### 7. Registry (`/registry`)

**Purpose**: Manage VM images (kernels, rootfs, data volumes)

**Status**: Fully implemented

**Page Structure**:

```
┌─ Image Registry ──────────────────────────┐
│ [Filter by type ▼]  [+ Import Image]     │
├────────────────────────────────────────── ┤
│ Type   │ Name      │ Size   │ Created    │
├────────┼───────────┼────────┼──────────── ┤
│ kernel │ linux-6.1 │ 8.2 MB │ 2025-10-01│
│ rootfs │ alpine    │ 42 MB  │ 2025-10-01│
│ data   │ db-vol-1  │ 500 MB │ 2025-10-02│
└────────────────────────────────────────── ┘
```

**Features**:
- Filter by image type (kernel, rootfs, data)
- Create new volumes
- Import external images
- Delete images
- Display image metadata (SHA256, size, created date)

**Image Types**:
- `kernel`: Bootable Linux kernel
- `rootfs`: Root filesystem for VMs
- `data`: Data volumes for attachment to VMs

**Operations**:

**Create Registry Volume**:
```typescript
// Frontend input
{
  name: "my-data-vol",
  size_bytes: 1073741824,  // 1 GB
  type: "data"
}
```

**Import Registry Image**:
```typescript
// Frontend input
{
  type: "kernel",  // or "rootfs", "data"
  name: "custom-kernel",
  path: "/srv/images/vmlinux-custom",
  url: undefined  // or URL for downloading
}
```

**Delete Image**:
```http
DELETE /v1/images/{image_id}
```

**React Query Hooks Used**:
- `useRegistryImages()` - List all images
- `useImportRegistryImage()` - Import new image
- `useCreateRegistryVolume()` - Create new volume
- `useDeleteRegistryItem()` - Remove image

---

### 8. Settings (`/settings`)

**Purpose**: System and user configuration

**Status**: Implemented (basic)

**Potential Settings**:
- API base URL configuration
- WebSocket settings
- Theme preferences
- Default VM settings
- Polling intervals
- Debug/log levels

---

### 9. Help (`/help`)

**Purpose**: User guidance and documentation

**Status**: Implemented (basic)

**Content**:
- Quick start guide
- Common tasks
- Troubleshooting
- API documentation links

---

### 10. Functions (`/function`)

**Purpose**: Serverless/function management (future feature)

**Status**: Placeholder

**Future Capability**: Run functions in lightweight VMs

---

## Core Features & API Reference

### Feature 1: VM Lifecycle Management

#### 1.1 Create VM

**Endpoint**: `POST /v1/vms`

**Description**: Create a new virtual machine

**Request**:
```typescript
interface CreateVmReq {
  name: string;                        // Required: VM identifier
  vcpu: number;                        // Required: CPU count (min: 1)
  mem_mib: number;                     // Required: Memory in MiB (min: 256)
  kernel_image_id?: string;            // UUID of kernel image
  kernel_path?: string;                // Raw path to kernel (if MANAGER_ALLOW_IMAGE_PATHS=1)
  rootfs_image_id?: string;            // UUID of rootfs image
  rootfs_path?: string;                // Raw path to rootfs (if MANAGER_ALLOW_IMAGE_PATHS=1)
  source_snapshot_id?: string;         // Clone from snapshot
}
```

**Example Request**:
```bash
curl -X POST http://localhost:18080/v1/vms \
  -H "Content-Type: application/json" \
  -d '{
    "name": "web-server-01",
    "vcpu": 2,
    "mem_mib": 512,
    "kernel_image_id": "59e1c754-2210-4887-858c-f3c5de7d483b",
    "rootfs_image_id": "4196a86f-95f4-4609-af23-138ec331b0dc"
  }'
```

**Response (Success - 200)**:
```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
}
```

**Response (Error - 400/500)**:
```json
{
  "error": "Invalid VM name",
  "suggestion": "VM name must be 3-64 characters, alphanumeric + hyphens",
  "fault_message": "Validation failed"
}
```

**Status Codes**:
- `200 OK`: VM created successfully
- `400 Bad Request`: Invalid parameters (missing required fields, invalid values)
- `500 Internal Server Error`: Backend failure (DB error, host unavailable)

**React Query Hook**:
```typescript
const { mutate: createVM, isPending, isError } = useCreateVM()

createVM({
  name: "my-vm",
  vcpu: 2,
  mem_mib: 512,
  kernel_image_id: "uuid",
  rootfs_image_id: "uuid"
})
```

**Edge Cases**:
- ✓ VM name already exists: Will fail with 400
- ✓ Invalid vCPU count: Must be > 0
- ✓ Insufficient memory: Must be >= 256 MiB
- ✓ Image not found: Will fail with 404
- ✓ Host at capacity: Will fail with 503
- ✓ Both raw path and image ID provided: Image ID takes precedence
- ⚠️ Raw paths require `MANAGER_ALLOW_IMAGE_PATHS=1` env var

---

#### 1.2 List VMs

**Endpoint**: `GET /v1/vms`

**Description**: Retrieve all virtual machines

**Query Parameters**: None

**Response (Success - 200)**:
```json
{
  "items": [
    {
      "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
      "name": "web-server-01",
      "state": "running",
      "vcpu": 2,
      "mem_mib": 512,
      "kernel_path": "/srv/images/kernel.bin",
      "rootfs_path": "/srv/images/alpine-rootfs.ext4",
      "host_id": "host-uuid-1",
      "host_addr": "192.168.1.100",
      "tap": "tap0",
      "api_sock": "/run/firecracker/vm-001.sock",
      "log_path": "/var/log/fc/vm-001.log",
      "http_port": 8001,
      "fc_unit": "fc-vm-001.service",
      "created_at": "2025-10-15T10:30:00Z",
      "updated_at": "2025-10-15T10:35:00Z",
      "template_id": null,
      "source_snapshot_id": null
    }
  ]
}
```

**React Query Hook**:
```typescript
const { data: vms, isLoading, error } = useVMs()
// Auto-refreshes every 30 seconds
```

**Pagination**: Currently returns all VMs (no pagination)

---

#### 1.3 Get VM Details

**Endpoint**: `GET /v1/vms/{id}`

**Description**: Retrieve detailed information about a specific VM

**Path Parameters**:
- `id` (UUID): The VM ID

**Response (Success - 200)**:
```json
{
  "item": {
    "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "name": "web-server-01",
    "state": "running",
    "vcpu": 2,
    "mem_mib": 512,
    "kernel_path": "/srv/images/kernel.bin",
    "rootfs_path": "/srv/images/alpine-rootfs.ext4",
    "host_id": "host-uuid-1",
    "host_addr": "192.168.1.100",
    "tap": "tap0",
    "api_sock": "/run/firecracker/vm-001.sock",
    "log_path": "/var/log/fc/vm-001.log",
    "http_port": 8001,
    "fc_unit": "fc-vm-001.service",
    "created_at": "2025-10-15T10:30:00Z",
    "updated_at": "2025-10-15T10:35:00Z"
  }
}
```

**Response (Error - 404)**:
```json
{
  "error": "VM not found",
  "suggestion": "Check the VM ID and try again",
  "fault_message": "VM with ID 'xxx' does not exist"
}
```

**React Query Hook**:
```typescript
const { data: vm, isLoading, error } = useVM(vmId)
// Auto-refreshes every 10 seconds
// Only runs if vmId is provided (enabled: !!vmId)
```

---

#### 1.4 Start VM

**Endpoint**: `POST /v1/vms/{id}/start`

**Description**: Start a stopped VM

**Prerequisites**:
- VM must be in `stopped` state
- All required images must be available
- Host must have capacity

**Request Body**: Empty object `{}`

**Response (Success - 200)**:
```json
{
  "ok": true
}
```

**Status Codes**:
- `200 OK`: VM start initiated
- `400 Bad Request`: VM not in stoppable state
- `404 Not Found`: VM does not exist
- `500 Internal Server Error`: Failed to start VM

**React Query Hook**:
```typescript
const { mutate: updateVMState } = useVmStatePatch()

updateVMState({ 
  id: vmId, 
  action: 'start' 
})
```

**Edge Cases**:
- VM already running: Returns 400
- VM in transitional state (starting): Returns 400
- No available host: Returns 503
- Image corrupted/missing: Returns 502

---

#### 1.5 Stop VM

**Endpoint**: `POST /v1/vms/{id}/stop`

**Description**: Gracefully shut down a running VM

**Request Body**: Empty object `{}`

**Response (Success - 200)**:
```json
{
  "ok": true
}
```

**Behavior**:
- Sends ACPI shutdown signal to guest OS
- VM may take 10-30 seconds to stop
- Firecracker process remains until VM exits

**Status Codes**:
- `200 OK`: Shutdown signal sent
- `400 Bad Request`: VM not running
- `404 Not Found`: VM does not exist
- `500 Internal Server Error`: Failed to stop VM

**React Query Hook**:
```typescript
const { mutate: updateVMState } = useVmStatePatch()

updateVMState({ 
  id: vmId, 
  action: 'stop' 
})
```

**Edge Cases**:
- Guest OS doesn't respond to ACPI: VM may hang, requires force kill
- VM already stopped: Returns 400
- Network timeout during shutdown: Cleanup may be delayed

---

#### 1.6 Pause VM

**Endpoint**: `POST /v1/vms/{id}/pause`

**Description**: Pause a running VM (freeze execution)

**Request Body**: Empty object `{}`

**Response (Success - 200)**:
```json
{
  "ok": true
}
```

**Prerequisites**:
- VM must be in `running` state

**Status Codes**:
- `200 OK`: VM paused successfully
- `400 Bad Request`: VM must be running to pause
- `404 Not Found`: VM does not exist
- `500 Internal Server Error`: Failed to pause VM

**React Query Hook**:
```typescript
const { mutate: updateVMState } = useVmStatePatch()

updateVMState({ 
  id: vmId, 
  action: 'pause' 
})
```

**Use Cases**:
- Create snapshots of running state
- Preserve memory state temporarily
- Troubleshooting and debugging

**Edge Cases**:
- VM in transitional state: Returns 400
- I/O operations in progress: May be blocked

---

#### 1.7 Resume VM

**Endpoint**: `POST /v1/vms/{id}/resume`

**Description**: Resume a paused VM

**Request Body**: Empty object `{}`

**Response (Success - 200)**:
```json
{
  "ok": true
}
```

**Prerequisites**:
- VM must be in `paused` state

**Status Codes**:
- `200 OK`: VM resumed successfully
- `400 Bad Request`: VM must be paused to resume
- `404 Not Found`: VM does not exist
- `500 Internal Server Error`: Failed to resume VM

**React Query Hook**:
```typescript
const { mutate: updateVMState } = useVmStatePatch()

updateVMState({ 
  id: vmId, 
  action: 'resume' 
})
```

---

#### 1.8 Delete VM

**Endpoint**: `DELETE /v1/vms/{id}`

**Description**: Permanently delete a VM and clean up all resources

**Response (Success - 200)**:
```json
{
  "ok": true
}
```

**Cleanup Operations**:
- Stops the Firecracker process
- Removes TAP interface
- Unmounts drives
- Deletes systemd scope
- Removes sockets and logs

**Status Codes**:
- `200 OK`: VM deleted successfully
- `404 Not Found`: VM does not exist
- `500 Internal Server Error`: Failed to delete VM

**React Query Hook**:
```typescript
const { mutate: deleteVM, isPending } = useDeleteVM()

deleteVM(vmId)
```

**Edge Cases**:
- VM running: Will stop first, then delete
- Orphaned resources: Reconciler cleans up after 30s
- Concurrent delete requests: Second request returns 404

**Warning**: This action is irreversible. All data is lost.

---

#### 1.9 Send Ctrl+Alt+Del

**Endpoint**: `POST /v1/vms/{id}/ctrl-alt-del`

**Description**: Send Ctrl+Alt+Del signal to a running VM

**Request Body**: Empty object `{}`

**Response (Success - 200)**:
```json
{
  "ok": true
}
```

**Prerequisites**:
- VM must be in `running` state

**Status Codes**:
- `200 OK`: Signal sent
- `400 Bad Request`: VM must be running
- `404 Not Found`: VM does not exist
- `500 Internal Server Error`: Failed to send signal

**React Query Hook**:
```typescript
const { mutate: updateVMState } = useVmStatePatch()

updateVMState({ 
  id: vmId, 
  action: 'ctrl_alt_del' 
})
```

**Guest Behavior**: Typically triggers reboot sequence in guest OS

---

### Feature 2: Drive (Storage) Management

#### 2.1 List VM Drives

**Endpoint**: `GET /v1/vms/{id}/drives`

**Description**: Get all storage drives attached to a VM

**Response (Success - 200)**:
```json
{
  "items": [
    {
      "id": "drive-uuid-1",
      "vm_id": "vm-uuid-1",
      "drive_id": "rootfs",
      "path_on_host": "/srv/images/alpine-rootfs.ext4",
      "is_root_device": true,
      "is_read_only": false,
      "size_bytes": 10737418240,
      "cache_type": "Unsafe",
      "io_engine": "Sync",
      "rate_limiter": null,
      "created_at": "2025-10-15T10:30:00Z",
      "updated_at": "2025-10-15T10:30:00Z"
    },
    {
      "id": "drive-uuid-2",
      "vm_id": "vm-uuid-1",
      "drive_id": "data-1",
      "path_on_host": "/srv/volumes/data-vol-01.ext4",
      "is_root_device": false,
      "is_read_only": false,
      "size_bytes": 1073741824,
      "cache_type": "Unsafe",
      "io_engine": "Sync",
      "rate_limiter": {
        "bandwidth": {
          "size": 1000000,
          "refill_time": 100
        }
      },
      "created_at": "2025-10-15T10:35:00Z",
      "updated_at": "2025-10-15T10:35:00Z"
    }
  ]
}
```

**React Query Hook**:
```typescript
const { data: drives } = useVMDrives(vmId)
```

---

#### 2.2 Create Drive

**Endpoint**: `POST /v1/vms/{id}/drives`

**Description**: Add a new storage drive to a VM

**Prerequisites**:
- VM must exist
- Drive file must exist on host
- VM should be stopped (for pre-boot drives)

**Request**:
```typescript
interface CreateDriveReq {
  drive_id: string;              // Required: Unique identifier for this drive
  path_on_host?: string;         // Path to block device/file on host
  is_root_device: boolean;       // Is this the root filesystem?
  is_read_only: boolean;         // Mount as read-only?
  size_bytes?: number;           // Size in bytes (optional)
  cache_type?: string;           // "Unsafe" | "Writeback" | "Writethrough"
  io_engine?: string;            // "Sync" | "Async"
  rate_limiter?: {               // Optional rate limiting
    bandwidth?: {
      size: number;              // Bytes per second
      refill_time: number;       // Milliseconds
    }
  }
}
```

**Example Request**:
```bash
curl -X POST http://localhost:18080/v1/vms/{vm_id}/drives \
  -H "Content-Type: application/json" \
  -d '{
    "drive_id": "data-2",
    "path_on_host": "/srv/volumes/data-vol-02.ext4",
    "is_root_device": false,
    "is_read_only": false,
    "size_bytes": 1073741824,
    "cache_type": "Unsafe",
    "io_engine": "Sync"
  }'
```

**Response (Success - 200)**:
```json
{
  "id": "drive-uuid-2",
  "vm_id": "vm-uuid-1",
  "drive_id": "data-2",
  "path_on_host": "/srv/volumes/data-vol-02.ext4",
  "is_root_device": false,
  "is_read_only": false,
  "size_bytes": 1073741824,
  "cache_type": "Unsafe",
  "io_engine": "Sync",
  "created_at": "2025-10-15T10:40:00Z",
  "updated_at": "2025-10-15T10:40:00Z"
}
```

**Status Codes**:
- `200 OK`: Drive created
- `400 Bad Request`: Invalid parameters or duplicate drive_id
- `404 Not Found`: VM not found
- `500 Internal Server Error`: Backend failure

**React Query Hook**:
```typescript
const { mutate: createDrive } = useCreateVMDrive()

createDrive({
  vmId: vmId,
  drive: {
    drive_id: "data-1",
    path_on_host: "/srv/volumes/data.ext4",
    is_root_device: false,
    is_read_only: false
  }
})
```

**Edge Cases**:
- File doesn't exist on host: Returns 400
- Duplicate drive_id: Returns 400
- VM running: May fail depending on backend state
- Filesystem errors on host: Returns 500

---

#### 2.3 Update Drive

**Endpoint**: `PATCH /v1/vms/{id}/drives/{drive_id}`

**Description**: Modify drive configuration (primarily rate limiting)

**Request**:
```typescript
interface UpdateDriveReq {
  path_on_host?: string;
  rate_limiter?: {
    bandwidth?: {
      size: number;
      refill_time: number;
    }
  }
}
```

**Example - Update Rate Limiting**:
```bash
curl -X PATCH http://localhost:18080/v1/vms/{vm_id}/drives/{drive_id} \
  -H "Content-Type: application/json" \
  -d '{
    "rate_limiter": {
      "bandwidth": {
        "size": 5000000,
        "refill_time": 100
      }
    }
  }'
```

**Response (Success - 200)**: Updated drive object

**React Query Hook**:
```typescript
const { mutate: updateDrive } = useUpdateVMDrive()

updateDrive({
  vmId: vmId,
  driveId: driveId,
  drive: { rate_limiter: {...} }
})
```

**Limitations**:
- ⚠️ Rate limiting may require VM to be paused
- ⚠️ Path changes not fully supported yet

---

#### 2.4 Delete Drive

**Endpoint**: `DELETE /v1/vms/{id}/drives/{drive_id}`

**Description**: Remove a drive from a VM

**Prerequisites**:
- VM should be stopped
- Not the root device (typically)

**Response (Success - 200)**:
```json
{
  "ok": true
}
```

**Status Codes**:
- `200 OK`: Drive deleted
- `400 Bad Request`: Cannot delete root device
- `404 Not Found`: Drive not found

**React Query Hook**:
```typescript
const { mutate: deleteDrive } = useDeleteVMDrive()

deleteDrive({
  vmId: vmId,
  driveId: driveId
})
```

---

### Feature 3: Network Interface Management

#### 3.1 List VM NICs

**Endpoint**: `GET /v1/vms/{id}/nics`

**Description**: Get all network interfaces attached to a VM

**Response (Success - 200)**:
```json
{
  "items": [
    {
      "id": "nic-uuid-1",
      "vm_id": "vm-uuid-1",
      "iface_id": "eth0",
      "host_dev_name": "tap0",
      "guest_mac": "aa:fc:00:00:00:01",
      "rx_rate_limiter": null,
      "tx_rate_limiter": null,
      "created_at": "2025-10-15T10:30:00Z",
      "updated_at": "2025-10-15T10:30:00Z"
    }
  ]
}
```

**React Query Hook**:
```typescript
const { data: nics } = useVMNics(vmId)
```

---

#### 3.2 Create NIC

**Endpoint**: `POST /v1/vms/{id}/nics`

**Description**: Add a network interface to a VM

**Request**:
```typescript
interface CreateNicReq {
  iface_id: string;              // Required: Interface ID (eth0, eth1, etc.)
  host_dev_name: string;         // Required: TAP device name (tap0, tap1, etc.)
  guest_mac?: string;            // Optional: MAC address for guest
  rx_rate_limiter?: {            // Optional RX rate limit
    bandwidth?: {
      size: number;
      refill_time: number;
    }
  }
  tx_rate_limiter?: {            // Optional TX rate limit
    bandwidth?: {
      size: number;
      refill_time: number;
    }
  }
}
```

**Example Request**:
```bash
curl -X POST http://localhost:18080/v1/vms/{vm_id}/nics \
  -H "Content-Type: application/json" \
  -d '{
    "iface_id": "eth0",
    "host_dev_name": "tap0",
    "guest_mac": "aa:fc:00:00:00:01"
  }'
```

**Response (Success - 200)**:
```json
{
  "id": "nic-uuid-1",
  "vm_id": "vm-uuid-1",
  "iface_id": "eth0",
  "host_dev_name": "tap0",
  "guest_mac": "aa:fc:00:00:00:01",
  "rx_rate_limiter": null,
  "tx_rate_limiter": null,
  "created_at": "2025-10-15T10:30:00Z",
  "updated_at": "2025-10-15T10:30:00Z"
}
```

**React Query Hook**:
```typescript
const { mutate: createNic } = useCreateVMNic()

createNic({
  vmId: vmId,
  nic: {
    iface_id: "eth0",
    host_dev_name: "tap0",
    guest_mac: "aa:fc:00:00:00:01"
  }
})
```

**Edge Cases**:
- TAP device doesn't exist: Returns 400
- Duplicate iface_id: Returns 400
- Invalid MAC format: Returns 400

---

#### 3.3 Update NIC

**Endpoint**: `PATCH /v1/vms/{id}/nics/{nic_id}`

**Description**: Modify NIC configuration (rate limiting)

**Request**:
```typescript
interface UpdateNicReq {
  rx_rate_limiter?: {
    bandwidth?: {
      size: number;
      refill_time: number;
    }
  }
  tx_rate_limiter?: {
    bandwidth?: {
      size: number;
      refill_time: number;
    }
  }
}
```

**Example - Set RX/TX Limits to 10 Mbps**:
```bash
curl -X PATCH http://localhost:18080/v1/vms/{vm_id}/nics/{nic_id} \
  -H "Content-Type: application/json" \
  -d '{
    "rx_rate_limiter": {
      "bandwidth": {
        "size": 1250000,
        "refill_time": 100
      }
    },
    "tx_rate_limiter": {
      "bandwidth": {
        "size": 1250000,
        "refill_time": 100
      }
    }
  }'
```

---

#### 3.4 Delete NIC

**Endpoint**: `DELETE /v1/vms/{id}/nics/{nic_id}`

**Description**: Remove a network interface from a VM

**Response (Success - 200)**:
```json
{
  "ok": true
}
```

**React Query Hook**:
```typescript
const { mutate: deleteNic } = useDeleteVMNic()

deleteNic({
  vmId: vmId,
  nicId: nicId
})
```

---

### Feature 4: Snapshots

#### 4.1 List VM Snapshots

**Endpoint**: `GET /v1/vms/{id}/snapshots`

**Description**: Get all snapshots for a specific VM

**Response (Success - 200)**:
```json
{
  "items": [
    {
      "id": "snapshot-uuid-1",
      "vm_id": "vm-uuid-1",
      "name": "pre-migration-backup",
      "snapshot_path": "/srv/snapshots/snap-001.vmstate",
      "mem_path": "/srv/snapshots/snap-001.mem",
      "size_bytes": 536870912,
      "snapshot_type": "Full",
      "state": "completed",
      "parent_id": null,
      "track_dirty_pages": false,
      "created_at": "2025-10-15T10:00:00Z",
      "updated_at": "2025-10-15T10:00:00Z"
    },
    {
      "id": "snapshot-uuid-2",
      "vm_id": "vm-uuid-1",
      "name": "incremental-backup",
      "snapshot_path": "/srv/snapshots/snap-002.vmstate",
      "mem_path": "/srv/snapshots/snap-002.mem",
      "size_bytes": 50331648,
      "snapshot_type": "Diff",
      "state": "completed",
      "parent_id": "snapshot-uuid-1",
      "track_dirty_pages": true,
      "created_at": "2025-10-15T11:00:00Z",
      "updated_at": "2025-10-15T11:00:00Z"
    }
  ]
}
```

**React Query Hook**:
```typescript
const { data: snapshots } = useSnapshots(vmId)
// Auto-refreshes every 30 seconds
```

---

#### 4.2 Create Snapshot

**Endpoint**: `POST /v1/vms/{id}/snapshots`

**Description**: Create a snapshot of a VM's state

**Prerequisites**:
- VM must be in `running` or `paused` state
- Sufficient disk space for snapshot

**Request**:
```typescript
interface CreateSnapshotRequest {
  name?: string;                 // Optional: Human-readable name
  snapshot_type?: "Full" | "Diff";  // Full or differential snapshot
  track_dirty_pages?: boolean;   // Track dirty pages for incremental?
  parent_id?: string;            // Parent snapshot ID for differential
}
```

**Example Request**:
```bash
curl -X POST http://localhost:18080/v1/vms/{vm_id}/snapshots \
  -H "Content-Type: application/json" \
  -d '{
    "name": "pre-update-snapshot",
    "snapshot_type": "Full",
    "track_dirty_pages": false
  }'
```

**Response (Success - 200)**:
```json
{
  "id": "snapshot-uuid-3",
  "name": "pre-update-snapshot"
}
```

**Status Codes**:
- `200 OK`: Snapshot created
- `404 Not Found`: VM not found
- `500 Internal Server Error`: Snapshot operation failed
- `502 Bad Gateway`: Agent communication failed

**React Query Hook**:
```typescript
const { mutate: createSnapshot, isPending } = useCreateSnapshot()

createSnapshot({
  vmId: vmId,
  snapshot_path: "/srv/snapshots/snap-003.vmstate",
  mem_file_path: "/srv/snapshots/snap-003.mem",
  snapshot_type: "Full"
})
```

**Time Estimate**: ~1-5 seconds for a 512 MB VM full snapshot

---

#### 4.3 Get Snapshot Details

**Endpoint**: `GET /v1/snapshots/{id}`

**Description**: Get detailed information about a specific snapshot

**Response (Success - 200)**:
```json
{
  "item": {
    "id": "snapshot-uuid-1",
    "vm_id": "vm-uuid-1",
    "name": "pre-migration-backup",
    "snapshot_path": "/srv/snapshots/snap-001.vmstate",
    "mem_path": "/srv/snapshots/snap-001.mem",
    "size_bytes": 536870912,
    "snapshot_type": "Full",
    "state": "completed",
    "parent_id": null,
    "track_dirty_pages": false,
    "created_at": "2025-10-15T10:00:00Z",
    "updated_at": "2025-10-15T10:00:00Z"
  }
}
```

---

#### 4.4 Restore Snapshot

**Endpoint**: `POST /v1/snapshots/{id}/instantiate`

**Description**: Create a new VM from a snapshot

**Request**:
```typescript
interface InstantiateSnapshotReq {
  name?: string;  // Optional: Name for the new VM
}
```

**Example Request**:
```bash
curl -X POST http://localhost:18080/v1/snapshots/{snapshot_id}/instantiate \
  -H "Content-Type: application/json" \
  -d '{
    "name": "restored-vm-01"
  }'
```

**Response (Success - 200)**:
```json
{
  "id": "new-vm-uuid",
  "name": "restored-vm-01"
}
```

**Status Codes**:
- `200 OK`: VM instantiated from snapshot
- `404 Not Found`: Snapshot not found
- `502 Bad Gateway`: Failed to instantiate

**React Query Hook**:
```typescript
const { mutate: restoreSnapshot } = useRestoreSnapshot()

restoreSnapshot({
  vmId: sourceVmId,
  snapshotId: snapshotId
})
```

**Time Estimate**: ~1-3 seconds (much faster than full VM creation)

---

#### 4.5 Delete Snapshot

**Endpoint**: `DELETE /v1/snapshots/{id}`

**Description**: Remove a snapshot

**Response (Success - 200)**:
```json
{
  "ok": true
}
```

**Cleanup**: Removes snapshot_path and mem_path files

**React Query Hook**:
```typescript
const { mutate: deleteSnapshot } = useDeleteSnapshot()

deleteSnapshot({
  vmId: vmId,
  snapshotId: snapshotId
})
```

---

### Feature 5: Templates

#### 5.1 List Templates

**Endpoint**: `GET /v1/templates`

**Description**: Get all VM templates

**Response (Success - 200)**:
```json
{
  "items": [
    {
      "id": "template-uuid-1",
      "name": "ubuntu-22.04-base",
      "spec": {
        "vcpu": 2,
        "mem_mib": 512,
        "kernel_image_id": "59e1c754-2210-4887-858c-f3c5de7d483b",
        "rootfs_image_id": "4196a86f-95f4-4609-af23-138ec331b0dc",
        "kernel_path": null,
        "rootfs_path": null
      },
      "created_at": "2025-10-15T09:00:00Z",
      "updated_at": "2025-10-15T09:00:00Z"
    },
    {
      "id": "template-uuid-2",
      "name": "alpine-minimal",
      "spec": {
        "vcpu": 1,
        "mem_mib": 256,
        "kernel_image_id": "59e1c754-2210-4887-858c-f3c5de7d483b",
        "rootfs_image_id": "alpine-uuid",
        "kernel_path": null,
        "rootfs_path": null
      },
      "created_at": "2025-10-15T08:00:00Z",
      "updated_at": "2025-10-15T08:00:00Z"
    }
  ]
}
```

**React Query Hook**:
```typescript
const { data: templates } = useTemplates()
```

---

#### 5.2 Create Template

**Endpoint**: `POST /v1/templates`

**Description**: Create a new VM template

**Request**:
```typescript
interface CreateTemplateReq {
  name: string;
  spec: {
    vcpu: number;
    mem_mib: number;
    kernel_image_id?: string;
    rootfs_image_id?: string;
    kernel_path?: string;
    rootfs_path?: string;
  }
}
```

**Example Request**:
```bash
curl -X POST http://localhost:18080/v1/templates \
  -H "Content-Type: application/json" \
  -d '{
    "name": "web-server-template",
    "spec": {
      "vcpu": 4,
      "mem_mib": 1024,
      "kernel_image_id": "59e1c754-2210-4887-858c-f3c5de7d483b",
      "rootfs_image_id": "4196a86f-95f4-4609-af23-138ec331b0dc"
    }
  }'
```

**Response (Success - 200)**:
```json
{
  "id": "template-uuid-3"
}
```

**React Query Hook**:
```typescript
const { mutate: createTemplate } = useCreateTemplate()

createTemplate({
  name: "my-template",
  spec: {
    vcpu: 2,
    mem_mib: 512,
    kernel_image_id: "kernel-uuid",
    rootfs_image_id: "rootfs-uuid"
  }
})
```

---

#### 5.3 Get Template

**Endpoint**: `GET /v1/templates/{id}`

**Description**: Get template details

**Response (Success - 200)**:
```json
{
  "item": {
    "id": "template-uuid-1",
    "name": "ubuntu-22.04-base",
    "spec": {
      "vcpu": 2,
      "mem_mib": 512,
      "kernel_image_id": "59e1c754-2210-4887-858c-f3c5de7d483b",
      "rootfs_image_id": "4196a86f-95f4-4609-af23-138ec331b0dc"
    },
    "created_at": "2025-10-15T09:00:00Z",
    "updated_at": "2025-10-15T09:00:00Z"
  }
}
```

---

#### 5.4 Instantiate Template

**Endpoint**: `POST /v1/templates/{id}/instantiate`

**Description**: Create a new VM from a template (same as cloning)

**Request**:
```typescript
interface InstantiateTemplateReq {
  name: string;  // Name for the new VM
}
```

**Example Request**:
```bash
curl -X POST http://localhost:18080/v1/templates/{template_id}/instantiate \
  -H "Content-Type: application/json" \
  -d '{
    "name": "web-server-prod-01"
  }'
```

**Response (Success - 200)**:
```json
{
  "id": "new-vm-uuid"
}
```

**React Query Hook**:
```typescript
const { mutate: instantiateTemplate } = useInstantiateTemplate()

instantiateTemplate({
  id: templateId,
  params: {
    name: "web-server-prod-01"
  }
})
```

**Time Estimate**: ~1 second (launches new VM with template settings)

---

### Feature 6: Image Registry

#### 6.1 List Images

**Endpoint**: `GET /v1/images`

**Query Parameters**:
- `kind` (optional): Filter by type ("kernel", "rootfs", "data")
- `project` (optional): Filter by project
- `name` (optional): Filter by name

**Response (Success - 200)**:
```json
{
  "items": [
    {
      "id": "image-uuid-1",
      "kind": "kernel",
      "name": "linux-6.1-generic",
      "host_path": "/srv/images/kernel-6.1.bin",
      "sha256": "abc123def456...",
      "size": 8589934592,
      "project": "kernel-store",
      "created_at": "2025-10-10T08:00:00Z",
      "updated_at": "2025-10-10T08:00:00Z"
    },
    {
      "id": "image-uuid-2",
      "kind": "rootfs",
      "name": "alpine-3.18",
      "host_path": "/srv/images/alpine-3.18.ext4",
      "sha256": "xyz789uvw012...",
      "size": 44040192,
      "project": "rootfs-store",
      "created_at": "2025-10-10T09:00:00Z",
      "updated_at": "2025-10-10T09:00:00Z"
    }
  ]
}
```

**React Query Hook**:
```typescript
const { data: images } = useRegistryImages()

// Filter by kind
const kernels = await facadeApi.getImagesByKind('kernel')
const rootfs = await facadeApi.getImagesByKind('rootfs')
```

---

#### 6.2 Get Image Details

**Endpoint**: `GET /v1/images/{id}`

**Response (Success - 200)**:
```json
{
  "item": {
    "id": "image-uuid-1",
    "kind": "kernel",
    "name": "linux-6.1-generic",
    "host_path": "/srv/images/kernel-6.1.bin",
    "sha256": "abc123def456...",
    "size": 8589934592,
    "project": "kernel-store",
    "created_at": "2025-10-10T08:00:00Z",
    "updated_at": "2025-10-10T08:00:00Z"
  }
}
```

---

#### 6.3 Create Image (Import/Upload)

**Endpoint**: `POST /v1/images`

**Description**: Register a new image in the registry

**Request**:
```typescript
interface CreateImageReq {
  kind: string;           // Required: "kernel" | "rootfs" | "data"
  name: string;           // Required: Image name
  host_path: string;      // Required: Path on host to image file
  sha256: string;         // Required: SHA256 hash
  size: number;           // Required: Size in bytes
  project?: string;       // Optional: Project identifier
}
```

**Example Request**:
```bash
curl -X POST http://localhost:18080/v1/images \
  -H "Content-Type: application/json" \
  -d '{
    "kind": "kernel",
    "name": "custom-kernel-6.2",
    "host_path": "/srv/images/kernel-6.2.bin",
    "sha256": "abc123def456...",
    "size": 8589934592,
    "project": "production"
  }'
```

**Response (Success - 200)**:
```json
{
  "id": "image-uuid-3"
}
```

**Status Codes**:
- `200 OK`: Image registered
- `400 Bad Request`: Invalid path (not under MANAGER_IMAGE_ROOT)
- `500 Internal Server Error`: Failed to store metadata

**React Query Hook**:
```typescript
const { mutate: importImage } = useImportRegistryImage()

importImage({
  type: "kernel",
  name: "custom-kernel",
  path: "/srv/images/kernel.bin",
  url: undefined
})
```

**Path Security**:
- ⚠️ Paths must be within `MANAGER_IMAGE_ROOT` (default: `/srv/images`)
- ⚠️ Raw paths require `MANAGER_ALLOW_IMAGE_PATHS=1` env var
- ✓ Using image IDs is preferred (no path validation needed)

---

#### 6.4 Delete Image

**Endpoint**: `DELETE /v1/images/{id}`

**Description**: Remove an image from the registry

**Prerequisites**:
- No running VMs using this image
- No templates referencing this image

**Response (Success - 200)**:
```json
{
  "ok": true
}
```

**Status Codes**:
- `200 OK`: Image deleted
- `404 Not Found`: Image not found
- `500 Internal Server Error`: Failed to delete

**React Query Hook**:
```typescript
const { mutate: deleteImage } = useDeleteRegistryItem()

deleteImage(imageId)
```

**Note**: Does NOT delete the actual file on disk, only the registry entry

---

### Feature 7: Monitoring & Logs

#### 7.1 Flush VM Metrics

**Endpoint**: `POST /v1/vms/{id}/flush-metrics`

**Description**: Force flush of Firecracker metrics

**Request Body**: Empty object `{}`

**Response (Success - 200)**:
```json
{
  "ok": true
}
```

**Prerequisites**:
- VM must be running
- Metrics must be enabled (MANAGER_ENABLE_METRICS=1)

**React Query Hook**:
```typescript
const { mutate: flushMetrics } = useVmStatePatch()

flushMetrics({
  id: vmId,
  action: 'flush_metrics'
})
```

**Use Cases**:
- Force immediate metric collection
- Debugging performance issues

---

#### 7.2 Tail VM Logs

**Endpoint**: `GET /v1/logs/tail?path={log_file_path}`

**Description**: Read recent VM logs

**Query Parameters**:
- `path` (required): Path to log file (e.g., `/var/log/fc/vm-001.log`)

**Response (Success - 200)**:
```json
{
  "text": "[2025-10-15 10:30:00] Starting Firecracker...\n[2025-10-15 10:30:01] Loaded boot config\n[2025-10-15 10:30:02] VM started\n"
}
```

**Status Codes**:
- `200 OK`: Log content returned
- `400 Bad Request`: Invalid path
- `404 Not Found`: Log file not found

**Usage**:
```bash
curl "http://localhost:18080/v1/logs/tail?path=/var/log/fc/vm-001.log"
```

**Note**: This is a simple file read endpoint (dev only). For production, consider SSE or WebSocket streaming.

---

### Feature 8: Virtual Machine Shell Access

#### 8.1 WebSocket Terminal Connection

**WebSocket URL**: `ws://localhost:18081/v1/vms/{id}/shell/ws`

**Description**: Real-time shell access to a running VM

**Handshake**:
```
GET /v1/vms/{id}/shell/ws HTTP/1.1
Upgrade: websocket
Connection: Upgrade
```

**Message Format** (bidirectional):
```json
{
  "type": "output",
  "data": "Welcome to Alpine Linux..."
}
```

**Usage in Frontend**:
```typescript
const wsUrl = facadeApi.getShellWebSocketUrl(vmId)
const ws = new WebSocket(wsUrl)

ws.onmessage = (event) => {
  const data = JSON.parse(event.data)
  // Handle terminal output
}

ws.send(JSON.stringify({ type: "input", data: "ls -la\n" }))
```

---

## Data Models

### VM State Machine

```
[creating] → [stopped] → [running]
                ↓           ↓
              [deleted]  [paused]
                ↑           ↓
                └─────────────
```

**States**:
- `creating`: VM is being provisioned
- `running`: VM is executing
- `paused`: VM is frozen (execution halted)
- `stopped`: VM is shut down
- `deleted`: VM is removed

---

### Rate Limiter Configuration

```typescript
interface RateLimiter {
  bandwidth?: {
    size: number;           // Bytes per refill period
    refill_time: number;    // Milliseconds (typically 100ms)
  }
}
```

**Examples**:
```typescript
// 10 Mbps = 1,250,000 bytes per 100ms
{
  bandwidth: {
    size: 1250000,
    refill_time: 100
  }
}

// 100 Mbps = 12,500,000 bytes per 100ms
{
  bandwidth: {
    size: 12500000,
    refill_time: 100
  }
}
```

---

### Snapshot Types

| Type | Size | Speed | Use Case |
|------|------|-------|----------|
| `Full` | Large | Normal | First snapshot, full system backup |
| `Diff` | Small | Normal | Incremental snapshots, lower disk usage |

---

## Implementation Status

### Fully Implemented ✓

- VM lifecycle (Create, Start, Stop, Delete, Pause, Resume)
- List and get VM details
- Network interfaces (Create, List, Get, Update, Delete)
- Drives (Create, List, Get, Update, Delete)
- Snapshots (Create, List, Restore, Delete)
- Templates (Create, List, Get, Instantiate)
- Image Registry (List, Create, Delete)
- Metrics flush
- Log tailing

### Partially Implemented ⚠️

- Metrics visualization (framework ready, no live streaming)
- Terminal access (WebSocket endpoint ready, UI integration incomplete)
- Rate limiting (API ready, UI controls missing)

### Not Yet Implemented ❌

- File upload to registry
- Machine configuration updates post-creation
- Boot source updates
- Direct Firecracker balloon device management
- Direct MMDS configuration
- Advanced reconciliation policies
- API token authentication
- RBAC (role-based access control)
- Audit logging
- Bulk VM operations

---

## Edge Cases & Error Handling

### Common Error Scenarios

#### 1. VM Creation Fails

```json
{
  "error": "Failed to create VM",
  "suggestion": "Ensure host has available capacity",
  "fault_message": "Host 'host-1' has no available TAP devices"
}
```

**Recovery**:
- Check host capacity
- Verify image availability
- Ensure TAP devices available

#### 2. Network Interface Conflict

```json
{
  "error": "Cannot create NIC",
  "suggestion": "TAP device 'tap0' already in use",
  "fault_message": "Device conflict"
}
```

**Recovery**:
- Use different TAP device name
- Clean up orphaned interfaces

#### 3. Snapshot Fails

```json
{
  "error": "Snapshot creation failed",
  "suggestion": "Insufficient disk space for snapshot",
  "fault_message": "No space left on device"
}
```

**Recovery**:
- Free up disk space
- Delete old snapshots
- Increase storage

#### 4. Snapshot Already Exists

```json
{
  "error": "Snapshot ID already exists",
  "suggestion": "Use a different snapshot name",
  "fault_message": "Duplicate snapshot_id"
}
```

#### 5. VM in Transitional State

```json
{
  "error": "Cannot perform action",
  "suggestion": "VM is currently transitioning. Wait and retry.",
  "fault_message": "VM state is 'creating', expected 'stopped'"
}
```

**Recovery**: Retry after 5-10 seconds

### Timeout Scenarios

| Operation | Typical Duration | Max Wait |
|-----------|-----------------|----------|
| VM creation | 2-5 seconds | 30 seconds |
| VM start | 1-3 seconds | 10 seconds |
| VM stop | 1-10 seconds | 30 seconds |
| Snapshot create | 1-5 seconds | 60 seconds |
| Snapshot restore | 1-3 seconds | 30 seconds |

### Concurrent Operation Handling

- **Multiple creates of same VM**: Returns 409 Conflict on second request
- **Delete while VM running**: VM stops first, then deletes
- **Snapshot during network operation**: May queue or reject with 503
- **Snapshot during migration**: Not supported, returns 400

---

## API Error Response Format

All error responses follow this format:

```typescript
interface ErrorResponse {
  error: string;           // Error title
  suggestion?: string;     // Suggested fix
  fault_message?: string;  // Detailed error message
  status?: number;         // HTTP status code (optional)
}
```

---

## Rate Limiting Recommendations

| Operation | Rate | Burst |
|-----------|------|-------|
| VM creation | 10/min | 20 |
| VM state change | 30/min | 50 |
| Snapshot creation | 5/min | 10 |
| Image upload | 1/min | 2 |

---

## Next Steps for UI/UX Team

### High Priority

1. **VM Details Page**: Complete all tabs with mock data
2. **Create VM Wizard**: Add advanced step for drives/NICs
3. **Snapshot UI**: Build create/restore dialogs
4. **Error Handling**: Standardize error notifications

### Medium Priority

5. **Bulk Operations**: Select multiple VMs for batch actions
6. **Search/Filter**: Improve VM list filtering
7. **Rate Limiting UI**: Add sliders for bandwidth configuration
8. **Metrics Dashboard**: Build charts for CPU/memory/I/O

### Nice to Have

9. **Terminal Integration**: WebSocket-based shell access
10. **Real-time Updates**: WebSocket subscriptions for VM state
11. **Advanced Scheduling**: Cron-like VM provisioning
12. **Cost Estimation**: Show estimated costs for VM configs

---

## Support & Questions

For questions about these features or API details, refer to:
- OpenAPI Spec: `/openapi/manager/openapi.yaml`
- Frontend Queries: `/apps/frontend/lib/queries.ts`
- API Facade: `/apps/frontend/lib/api/facade.ts`
- Current State: `/CURRENT_STATE.md`
- Product Requirements: `/PRD.md`
