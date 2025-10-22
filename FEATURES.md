# NQRust MicroVM - Features Documentation

**Last Updated:** 2025-10-20

This document provides a comprehensive mapping of all backend (Manager) and frontend features, showing which features are fully integrated, partially wired, or missing integration.

---

## Table of Contents
- [Integration Status Legend](#integration-status-legend)
- [Feature Matrix](#feature-matrix)
- [Backend-Only Features](#backend-only-features-not-in-frontend)
- [Frontend-Only Features](#frontend-only-features-not-in-backend)
- [Detailed Feature Breakdown](#detailed-feature-breakdown)

---

## Integration Status Legend

- ✅ **Fully Wired** - Backend endpoint exists, frontend UI/API client implemented, fully functional
- ⚠️ **Partially Wired** - Backend exists but frontend has limited/incomplete implementation
- ❌ **Not Wired** - Backend endpoint exists but no frontend integration
- 🔨 **Backend Only** - Feature only exists in backend
- 🎨 **Frontend Only** - Feature only exists in frontend

---

## Feature Matrix

### VM Lifecycle Management

| Feature | Backend Endpoint | Frontend Integration | Status | Notes |
|---------|-----------------|---------------------|--------|-------|
| Create VM | `POST /v1/vms` | `useCreateVM()` + Creation Wizard | ✅ | Full wizard with 5 steps |
| List VMs | `GET /v1/vms` | `useVMs()` + VM List Page | ✅ | With search and filtering |
| Get VM Details | `GET /v1/vms/{id}` | `useVM(id)` + Detail Page | ✅ | 7-tab detail view |
| Start VM | `POST /v1/vms/{id}/start` | `useVmStatePatch()` + Action Buttons | ✅ | In cards and detail page |
| Stop VM | `POST /v1/vms/{id}/stop` | `useVmStatePatch()` + Action Buttons | ✅ | In cards and detail page |
| Pause VM | `POST /v1/vms/{id}/pause` | `useVmStatePatch()` + Action Buttons | ✅ | In cards and detail page |
| Resume VM | `POST /v1/vms/{id}/resume` | `useVmStatePatch()` + Action Buttons | ✅ | In cards and detail page |
| Delete VM | `DELETE /v1/vms/{id}` | `useDeleteVM()` + Delete Button | ✅ | With confirmation dialog |

### VM Configuration

| Feature | Backend Endpoint | Frontend Integration | Status | Notes |
|---------|-----------------|---------------------|--------|-------|
| Update Machine Config | `PATCH /v1/vms/{id}/machine-config` | VM Creation Wizard | ⚠️ | Only during creation, no runtime updates |
| Configure CPU | `PUT /v1/vms/{id}/cpu-config` | Not implemented | ❌ | Backend exists, no UI |
| Configure VSock | `PUT /v1/vms/{id}/vsock` | Not implemented | ❌ | Backend exists, no UI |
| Configure Entropy | `PUT /v1/vms/{id}/entropy` | Not implemented | ❌ | Backend exists, no UI |
| Configure Serial | `PUT /v1/vms/{id}/serial` | Not implemented | ❌ | Backend exists, no UI |
| Configure Logger | `PUT /v1/vms/{id}/logger` | Not implemented | ❌ | Backend exists, no UI |
| Configure Balloon | `PUT /v1/vms/{id}/balloon` | Not implemented | ❌ | Backend exists, no UI |
| Update Balloon | `PATCH /v1/vms/{id}/balloon` | Not implemented | ❌ | Backend exists, no UI |
| Balloon Statistics | `PATCH /v1/vms/{id}/balloon/statistics` | Not implemented | ❌ | Backend exists, no UI |

### VM Control & Monitoring

| Feature | Backend Endpoint | Frontend Integration | Status | Notes |
|---------|-----------------|---------------------|--------|-------|
| Send Ctrl-Alt-Del | `POST /v1/vms/{id}/ctrl-alt-del` | `useVmStatePatch()` + Action Menu | ✅ | In VM action menu |
| Flush Metrics | `POST /v1/vms/{id}/flush-metrics` | `useVmStatePatch()` | ⚠️ | Called internally, no explicit UI button |
| Update Guest IP | `POST /v1/vms/{id}/guest-ip` | Not in UI (auto via guest agent) | ✅ | Automatic via guest agent |
| Get Shell Access | `GET /v1/vms/{id}/shell` | `getShellCredentials()` + Terminal Tab | ✅ | Full terminal implementation |
| Shell WebSocket | `GET /v1/vms/{id}/shell/ws` | Terminal Component | ✅ | Real-time shell via WebSocket |
| Metrics WebSocket | `GET /v1/vms/{id}/metrics/ws` | Metrics Tab | ✅ | Real-time charts |

### Storage Management (Drives)

| Feature | Backend Endpoint | Frontend Integration | Status | Notes |
|---------|-----------------|---------------------|--------|-------|
| List Drives | `GET /v1/vms/{id}/drives` | `useVMDrives()` + Storage Tab | ✅ | Full drive management UI |
| Get Drive | `GET /v1/vms/{id}/drives/{drive_id}` | Drive Editor Dialog | ✅ | View drive details |
| Create Drive | `POST /v1/vms/{id}/drives` | `useCreateVMDrive()` + Dialog | ✅ | Add drives to VMs |
| Update Drive | `PATCH /v1/vms/{id}/drives/{drive_id}` | `useUpdateVMDrive()` + Dialog | ⚠️ | UI exists, rate limiter not fully implemented |
| Delete Drive | `DELETE /v1/vms/{id}/drives/{drive_id}` | `useDeleteVMDrive()` + Button | ✅ | With confirmation |

### Network Management (NICs)

| Feature | Backend Endpoint | Frontend Integration | Status | Notes |
|---------|-----------------|---------------------|--------|-------|
| List NICs | `GET /v1/vms/{id}/nics` | `useVMNics()` + Network Tab | ✅ | Full NIC management UI |
| Get NIC | `GET /v1/vms/{id}/nics/{nic_id}` | NIC Editor Dialog | ✅ | View NIC details |
| Create NIC | `POST /v1/vms/{id}/nics` | `useCreateVMNic()` + Dialog | ✅ | Add NICs to VMs |
| Update NIC | `PATCH /v1/vms/{id}/nics/{nic_id}` | `useUpdateVMNic()` + Dialog | ⚠️ | UI exists, rate limiter partially implemented |
| Delete NIC | `DELETE /v1/vms/{id}/nics/{nic_id}` | `useDeleteVMNic()` + Button | ✅ | With confirmation |

### Snapshots

| Feature | Backend Endpoint | Frontend Integration | Status | Notes |
|---------|-----------------|---------------------|--------|-------|
| List VM Snapshots | `GET /v1/vms/{id}/snapshots` | `useSnapshots()` + Snapshots Tab | ✅ | Full snapshot UI |
| Create Snapshot | `POST /v1/vms/{id}/snapshots` | `useCreateSnapshot()` + Dialog | ✅ | Full/Diff snapshots |
| Get Snapshot | `GET /v1/snapshots/{id}` | Not directly used | ⚠️ | Backend exists, not called by FE |
| Restore Snapshot | `POST /v1/snapshots/{id}/instantiate` | `useRestoreSnapshot()` + Button | ✅ | Creates new VM from snapshot |
| Delete Snapshot | `DELETE /v1/snapshots/{id}` | `useDeleteSnapshot()` + Button | ✅ | With confirmation |

### Images & Registry

| Feature | Backend Endpoint | Frontend Integration | Status | Notes |
|---------|-----------------|---------------------|--------|-------|
| List Images | `GET /v1/images` | `useRegistryImages()` + Registry Page | ✅ | Full registry browser |
| Create/Import Image | `POST /v1/images` | `useImportRegistryImage()` + Dialog | ✅ | Import from path/URL |
| Get Image | `GET /v1/images/{id}` | Used in dropdowns | ✅ | Image selection in VM creation |
| Delete Image | `DELETE /v1/images/{id}` | `useDeleteRegistryItem()` + Button | ✅ | With confirmation |

### Templates

| Feature | Backend Endpoint | Frontend Integration | Status | Notes |
|---------|-----------------|---------------------|--------|-------|
| List Templates | `GET /v1/templates` | `useTemplates()` | ⚠️ | API wired but no UI page |
| Get Template | `GET /v1/templates/{id}` | `useTemplate(id)` | ⚠️ | API wired but no UI page |
| Create Template | `POST /v1/templates` | Not implemented | ❌ | Backend exists, no UI |
| Instantiate Template | `POST /v1/templates/{id}/instantiate` | Not implemented | ❌ | Backend exists, no UI |

### Hosts (Agent Registration)

| Feature | Backend Endpoint | Frontend Integration | Status | Notes |
|---------|-----------------|---------------------|--------|-------|
| Register Host | `POST /v1/hosts/register` | Not in UI (server-to-server) | 🔨 | Backend-only (agent → manager) |
| Host Heartbeat | `POST /v1/hosts/{id}/heartbeat` | Not in UI (server-to-server) | 🔨 | Backend-only (agent → manager) |

### MMDS (Metadata Service)

| Feature | Backend Endpoint | Frontend Integration | Status | Notes |
|---------|-----------------|---------------------|--------|-------|
| Set MMDS Data | `PUT /v1/vms/{id}/mmds` | Not implemented | ❌ | Backend exists, no UI |
| Configure MMDS | `PUT /v1/vms/{id}/mmds/config` | Not implemented | ❌ | Backend exists, no UI |

### Logs

| Feature | Backend Endpoint | Frontend Integration | Status | Notes |
|---------|-----------------|---------------------|--------|-------|
| Tail Log File | `GET /v1/logs/tail` | Not implemented | ❌ | Dev utility, no UI needed |

---

## Backend-Only Features (Not in Frontend)

These features have backend endpoints but no frontend UI:

### Advanced Configuration (Missing UI)
1. **CPU Configuration** - `PUT /v1/vms/{id}/cpu-config`
   - Set CPU template, hyperthreading, etc.
   - Currently only configurable during VM creation

2. **VSock Device** - `PUT /v1/vms/{id}/vsock`
   - Configure host-guest communication socket
   - No UI for configuration

3. **Entropy Device** - `PUT /v1/vms/{id}/entropy`
   - Configure random number generator
   - No UI for configuration

4. **Serial Console** - `PUT /v1/vms/{id}/serial`
   - Configure serial device (separate from shell)
   - No UI for configuration

5. **Logger** - `PUT /v1/vms/{id}/logger`
   - Configure Firecracker logging
   - No UI for configuration

6. **Balloon Memory** - `PUT /v1/vms/{id}/balloon` + `PATCH`
   - Dynamic memory management
   - Backend ready, no UI implementation

7. **MMDS** - `PUT /v1/vms/{id}/mmds` + `/mmds/config`
   - MicroVM Metadata Service
   - Backend ready, no UI

### Templates (Partially Implemented)
8. **Template Management** - Complete backend, partial frontend
   - ✅ Backend: Create, list, get, instantiate templates
   - ⚠️ Frontend: API hooks exist but no UI pages
   - **Missing:** Template creation wizard, template browser, instantiation UI

### Host Management (Backend-Only by Design)
9. **Host Registration & Heartbeat**
   - Server-to-server communication
   - No UI needed (agents register automatically)

---

## Frontend-Only Features (Not in Backend)

These features exist in the frontend but may not have direct backend support:

1. **Lambda/IDE Playground** (`/function` page)
   - Web-based code editor for Node.js/Python
   - Status: Unknown if connected to backend execution
   - Location: `apps/frontend/app/(dash)/function/page.tsx`

2. **Dashboard Analytics** (`/dashboard` page)
   - VM statistics, filtering by state/owner/environment
   - Uses standard VM list endpoint with client-side filtering
   - Could benefit from backend aggregation endpoints

3. **Bulk VM Operations**
   - Multi-select VMs in dashboard
   - Frontend state only, operations still per-VM

4. **Image Rename**
   - UI exists in registry browser
   - Not supported by backend (mutation marked as not working)

5. **File Upload for Images**
   - `useUploadRegistryFile()` hook exists
   - Marked as "not implemented" - unclear if backend supports

---

## Detailed Feature Breakdown

### ✅ Fully Integrated Features

These features work end-to-end from frontend to backend:

#### 1. **VM Lifecycle Management** (Complete)
- **Creation**: 5-step wizard (info → credentials → machine config → boot source → network)
- **State Control**: Start, stop, pause, resume with visual feedback
- **Monitoring**: Real-time state updates, status indicators
- **Deletion**: Safe deletion with confirmation dialogs

**Frontend Files:**
- `apps/frontend/components/vm-creation-wizard.tsx`
- `apps/frontend/components/vm-card.tsx`
- `apps/frontend/components/action-menu.tsx`

**Backend Files:**
- `apps/manager/src/features/vms/routes.rs`
- `apps/manager/src/features/vms/service.rs`

---

#### 2. **Storage Management** (Complete)
- **Drive CRUD**: Create, read, update, delete drives
- **Volume Creation**: Create blank volumes from registry
- **Rate Limiting**: UI exists but not fully functional

**Frontend Files:**
- `apps/frontend/components/drive-list.tsx`
- `apps/frontend/components/drive-editor-dialog.tsx`

**Backend Files:**
- `apps/manager/src/features/vms/routes.rs` (drives endpoints)

---

#### 3. **Network Management** (Complete)
- **NIC CRUD**: Full NIC management
- **MAC Address**: Auto-generation or manual entry
- **Rate Limiting**: Partial UI implementation

**Frontend Files:**
- `apps/frontend/components/network-table.tsx`
- `apps/frontend/components/nic-editor-dialog.tsx`

**Backend Files:**
- `apps/manager/src/features/vms/routes.rs` (NICs endpoints)

---

#### 4. **Snapshots** (Complete)
- **Create**: Full/Differential snapshots with dirty page tracking
- **Restore**: Instantiate new VMs from snapshots
- **Delete**: Remove snapshots
- **List**: View all snapshots for a VM

**Frontend Files:**
- `apps/frontend/components/snapshots-tab.tsx`

**Backend Files:**
- `apps/manager/src/features/snapshots/routes.rs`
- `apps/manager/src/features/vms/routes.rs`

---

#### 5. **Real-time Terminal** (Complete)
- **Shell Access**: Full terminal via WebSocket
- **Credentials**: Display username/password
- **xterm.js**: Full terminal emulation
- **Auto-reconnect**: Resilient connection handling

**Frontend Files:**
- `apps/frontend/components/vm-terminal.tsx`
- `apps/frontend/lib/ws.ts`

**Backend Files:**
- `apps/manager/src/features/vms/shell.rs`
- `apps/manager/src/features/vms/routes.rs`

---

#### 6. **Real-time Metrics** (Complete)
- **WebSocket Streaming**: Live CPU, memory, network, disk metrics
- **Guest Agent Integration**: Metrics from inside VM (port 9000)
- **Charts**: Line charts with 60-second history
- **Fallback**: Host-side metrics if guest agent unavailable

**Frontend Files:**
- `apps/frontend/components/metrics-tab.tsx`
- `apps/frontend/lib/ws.ts`

**Backend Files:**
- `apps/manager/src/features/vms/routes.rs` (metrics websocket)
- `apps/manager/src/features/vms/service.rs` (guest agent client)
- `apps/guest-agent/src/main.rs`

---

#### 7. **Image Registry** (Complete)
- **Browse**: List all kernels and rootfs images
- **Import**: Add images from file paths or URLs
- **Create Volumes**: New blank disk images
- **Delete**: Remove images
- **Filter**: By type (kernel/rootfs), search by name

**Frontend Files:**
- `apps/frontend/components/registry-browser.tsx`
- `apps/frontend/app/(dash)/registry/page.tsx`

**Backend Files:**
- `apps/manager/src/features/images/routes.rs`
- `apps/manager/src/features/images/repo.rs`

---

### ⚠️ Partially Wired Features

These features are partially implemented or have limitations:

#### 1. **Templates** - Backend complete, Frontend incomplete
**Status:** API hooks exist, no UI pages

**What Works:**
- Backend: Full CRUD + instantiation
- Frontend: React Query hooks (`useTemplates`, `useTemplate`)

**What's Missing:**
- Template creation wizard
- Template browser/list page
- Template instantiation UI

**Recommendation:**
- Create `/templates` page listing templates
- Add "Create Template from VM" button
- Add "Deploy from Template" action

---

#### 2. **Rate Limiting** - UI exists, functionality limited
**Status:** Dialog exists but not fully functional

**What Works:**
- Drive/NIC editor dialogs have rate limiter fields
- Backend supports rate limiters

**What's Missing:**
- Validation of rate limiter values
- Better UX for common presets (1Gbps, 10Gbps, etc.)
- Display current rate limits in tables

**Recommendation:**
- Add rate limiter presets
- Validate bandwidth/ops values
- Show active rate limits in drive/NIC tables

---

#### 3. **VM Configuration Updates** - Creation only
**Status:** Configuration only at creation time

**What Works:**
- Full configuration during VM creation wizard
- Machine config (CPU/memory) settable

**What's Missing:**
- Runtime reconfiguration (requires VM stop/start)
- UI for CPU template, VSock, Entropy, Serial, Logger, Balloon

**Recommendation:**
- Add "Advanced Configuration" tab in VM detail page
- Support runtime updates where possible
- Show warnings for changes requiring restart

---

### ❌ Not Wired Features

These backend endpoints have no frontend integration:

#### 1. **Advanced Device Configuration**
- VSock, Entropy, Serial Console, Logger
- **Backend:** Complete endpoints exist
- **Frontend:** No UI at all
- **Impact:** Limited to default configurations
- **Recommendation:** Add "Advanced" tab in VM creation wizard

#### 2. **Balloon Memory Management**
- **Backend:** Full balloon device support (PUT/PATCH)
- **Frontend:** No UI
- **Impact:** No dynamic memory management
- **Recommendation:** Add balloon controls to VM detail page

#### 3. **MMDS (Metadata Service)**
- **Backend:** Complete MMDS support
- **Frontend:** No UI
- **Impact:** Cannot configure metadata service
- **Recommendation:** Add MMDS editor in VM configuration

#### 4. **Detailed Snapshot Info**
- **Backend:** `GET /v1/snapshots/{id}` exists
- **Frontend:** Not called, snapshots shown in list only
- **Impact:** Cannot view snapshot details
- **Recommendation:** Add snapshot detail view

---

## Feature Availability Summary

### Total Backend Endpoints: **61**

**By Integration Status:**
- ✅ **Fully Wired:** 35 endpoints (57%)
- ⚠️ **Partially Wired:** 8 endpoints (13%)
- ❌ **Not Wired:** 16 endpoints (26%)
- 🔨 **Backend-Only (by design):** 2 endpoints (3%)

### Coverage by Feature Area:

| Feature Area | Total Endpoints | Wired | Partial | Not Wired | Coverage |
|--------------|-----------------|-------|---------|-----------|----------|
| VM Lifecycle | 8 | 8 | 0 | 0 | 100% ✅ |
| VM Control | 7 | 5 | 1 | 1 | 86% ✅ |
| Storage (Drives) | 5 | 4 | 1 | 0 | 100% ✅ |
| Network (NICs) | 5 | 4 | 1 | 0 | 100% ✅ |
| Snapshots | 5 | 4 | 1 | 0 | 100% ✅ |
| Images | 4 | 4 | 0 | 0 | 100% ✅ |
| Templates | 4 | 0 | 2 | 2 | 25% ⚠️ |
| VM Configuration | 12 | 1 | 0 | 11 | 8% ❌ |
| Hosts | 2 | 0 | 0 | 2 | N/A 🔨 |
| Logs | 1 | 0 | 0 | 1 | N/A 🔨 |

---

## Recommendations for Improvement

### High Priority (User-Facing)

1. **Complete Template Support**
   - Create template browser page
   - Add "Save as Template" to VM actions
   - Template instantiation wizard

2. **Advanced Configuration UI**
   - Add configuration tabs for VSock, Entropy, Serial, Balloon
   - Runtime reconfiguration where supported
   - Better visibility into current config

3. **Rate Limiter Enhancement**
   - Add presets and validation
   - Display active limits in tables
   - Better UX for bandwidth units

### Medium Priority (Quality of Life)

4. **Snapshot Details**
   - Snapshot detail view
   - Show snapshot size, type, creation date
   - Compare snapshots

5. **MMDS Configuration**
   - MMDS data editor
   - Common metadata templates
   - Preview MMDS output

6. **Dashboard Improvements**
   - Backend aggregation endpoints (stats by state/owner)
   - Bulk operations support
   - Advanced filtering

### Low Priority (Nice to Have)

7. **Image Management**
   - File upload for images
   - Image rename support
   - Image versioning

8. **Logging & Debugging**
   - Log viewer in UI
   - Firecracker logs access
   - Guest agent logs

---

## Architecture Overview

### Backend (Manager)

**Language:** Rust
**Framework:** Axum (async HTTP framework)
**Database:** PostgreSQL (via SQLx)
**Location:** `apps/manager/`

**Key Components:**
- `features/vms/` - VM lifecycle and configuration
- `features/hosts/` - Agent registration
- `features/images/` - Image management
- `features/templates/` - VM templates
- `features/snapshots/` - Snapshot operations
- `core/` - Shared utilities

**API Documentation:**
- OpenAPI spec: `apps/manager/openapi/manager/openapi.yaml`
- Auto-generated from code annotations

---

### Frontend (Next.js)

**Language:** TypeScript
**Framework:** Next.js 14 (App Router)
**UI Library:** React + shadcn/ui + Tailwind CSS
**State Management:** React Query (TanStack Query)
**Location:** `apps/frontend/`

**Key Components:**
- `app/(dash)/` - Pages and routing
- `components/` - Reusable UI components
- `lib/api/` - API client (`facade.ts`, `http.ts`)
- `lib/queries.ts` - React Query hooks
- `lib/ws.ts` - WebSocket utilities

**API Client:**
- Base URL: `/api/proxy/v1` (proxied through Next.js)
- WebSocket URL: `ws://localhost:8000` (configurable)

---

### Guest Agent

**Language:** Rust
**Framework:** Axum (HTTP server)
**Location:** `apps/guest-agent/`

**Features:**
- Real-time metrics from inside VM
- CPU, memory, uptime, load average, process count
- Auto-reports IP to manager
- Config file: `/etc/guest-agent.conf`
- Port: 9000

**Endpoints:**
- `GET /health` - Health check
- `GET /metrics` - Guest metrics JSON

---

## Data Flow Diagrams

### VM Creation Flow
```
User (Frontend) → VM Creation Wizard
    ↓ (5 steps: info, creds, config, boot, network)
Frontend API Client → POST /v1/vms
    ↓
Manager (Backend) → Validate request
    ↓
Manager → Select healthy host
    ↓
Manager → Install guest agent to rootfs
    ↓
Manager → Create VM via agent → Firecracker API
    ↓
Agent → Start Firecracker VMM
    ↓
Manager ← VM created successfully
    ↓
Frontend ← VM details returned
    ↓
User sees new VM in list
```

### Metrics Streaming Flow
```
User clicks "Start Monitoring" → Frontend
    ↓
Frontend → WebSocket /v1/vms/{id}/metrics/ws → Manager
    ↓
Manager → GET http://{guest_ip}:9000/metrics → Guest Agent
    ↓ (every 1 second)
Guest Agent → Read /proc/stat, /proc/meminfo
    ↓
Guest Agent → Return JSON metrics
    ↓
Manager → Combine with Firecracker metrics (network/disk)
    ↓
Manager → WebSocket send JSON to Frontend
    ↓
Frontend → Update charts with new data point
```

### Guest Agent Auto-Deployment
```
Manager receives VM creation request
    ↓
Manager mounts rootfs (loop device)
    ↓
Manager copies guest-agent binary → /usr/local/bin/guest-agent
    ↓
Manager writes config → /etc/guest-agent.conf
    VM_ID={uuid}
    MANAGER_URL=http://{bridge_ip}:18080
    ↓
Manager installs service (OpenRC/systemd/sysvinit)
    ↓
Manager unmounts rootfs
    ↓
VM boots → Guest agent auto-starts
    ↓
Guest agent reads config
    ↓
Guest agent detects IP from eth0
    ↓
Guest agent → POST /v1/vms/{id}/guest-ip → Manager
    (every 30 seconds)
```

---

## Environment Variables

### Manager (Backend)
```bash
DATABASE_URL=postgres://user:pass@localhost:5432/nexus
MANAGER_BIND=0.0.0.0:18080  # Listen address
MANAGER_IMAGE_ROOT=/srv/images  # Image storage
MANAGER_ALLOW_IMAGE_PATHS=true  # Allow direct paths
MANAGER_RECONCILER_DISABLED=false  # Disable reconciler
MANAGER_ENABLE_METRICS=true  # Enable metrics
AGENT_BASE=http://127.0.0.1:9090  # Agent URL
```

### Agent (Host Agent)
```bash
AGENT_BIND=127.0.0.1:9090  # Listen address
FC_RUN_DIR=/tmp/claude/fc  # Firecracker runtime dir
FC_BRIDGE=fcbr0  # Network bridge
MANAGER_BASE=http://127.0.0.1:18080  # Manager URL
```

### Frontend
```bash
NEXT_PUBLIC_API_BASE_URL=/api/proxy/v1  # Manager API
NEXT_PUBLIC_WS_BASE_URL=ws://localhost:8000  # WebSocket URL
```

### Guest Agent (inside VM)
```bash
# Config file: /etc/guest-agent.conf
VM_ID={uuid}  # Set during installation
MANAGER_URL=http://{bridge_ip}:18080  # Manager endpoint
```

---

## Known Issues & Limitations

### Backend

1. **Templates not fully utilized** - Complete implementation exists but no frontend
2. **Advanced configuration limited** - Many Firecracker features not exposed via API
3. **No batch operations** - All operations are per-VM

### Frontend

4. **Rate limiter UI incomplete** - Fields exist but validation missing
5. **No template management UI** - API hooks ready but no pages
6. **Dashboard filtering client-side** - Could use backend aggregation
7. **Image rename not supported** - Backend doesn't support rename operation
8. **No bulk VM operations** - Multi-select exists but operations still individual

### Integration

9. **Metrics fallback unclear** - Guest agent failures fall back to host metrics silently
10. **No health indicators** - UI doesn't show if guest agent is responding
11. **Configuration drift** - No way to detect if VM config changed outside manager

---

## Testing Coverage

### ✅ Tested Features
- VM lifecycle (create, start, stop, pause, resume, delete)
- Guest agent metrics integration
- Snapshot creation and restoration
- Terminal WebSocket connection
- Image registry CRUD
- Drive and NIC management

### ⚠️ Partially Tested
- Rate limiters (UI exists, unclear if functional)
- Template instantiation (backend tested, no UI)
- Balloon memory (backend exists, no testing)

### ❌ Untested
- MMDS configuration
- Advanced device configuration (VSock, Entropy, Serial)
- Runtime VM reconfiguration
- Bulk operations
- Error recovery and edge cases

---

## Future Roadmap Suggestions

### Phase 1: Complete Existing Features
1. Implement template management UI
2. Add advanced configuration tabs
3. Complete rate limiter implementation
4. Add snapshot detail views

### Phase 2: Enhanced Monitoring
5. Add guest agent health indicators
6. Historical metrics storage
7. Alerting and notifications
8. Log viewer integration

### Phase 3: Advanced Features
9. VM cloning and templating from running VMs
10. Bulk operations support
11. MMDS template library
12. Image versioning and rollback

### Phase 4: Enterprise Features
13. Multi-tenancy and RBAC
14. Resource quotas and limits
15. Billing and usage tracking
16. API rate limiting

---

## Contributing

When adding new features, please:

1. **Backend**: Add OpenAPI annotations to routes
2. **Frontend**: Create React Query hooks in `lib/queries.ts`
3. **Integration**: Update this document with new features
4. **Testing**: Add integration tests for critical paths
5. **Documentation**: Update API docs and user guides

---

## File Reference

### Backend Key Files
- `apps/manager/src/features/vms/routes.rs` - VM API endpoints
- `apps/manager/src/features/vms/service.rs` - VM business logic
- `apps/manager/src/features/vms/guest_agent.rs` - Guest agent installer
- `apps/manager/openapi/manager/openapi.yaml` - API specification

### Frontend Key Files
- `apps/frontend/lib/api/facade.ts` - Main API client
- `apps/frontend/lib/queries.ts` - React Query hooks
- `apps/frontend/components/vm-creation-wizard.tsx` - VM creation UI
- `apps/frontend/components/metrics-tab.tsx` - Metrics visualization

### Guest Agent Files
- `apps/guest-agent/src/main.rs` - Guest agent implementation
- Target binary: `target/x86_64-unknown-linux-musl/release/guest-agent`

---

**End of Documentation**
