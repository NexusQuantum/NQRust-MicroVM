# NQRust MicroVM Platform - Complete Frontend Specification

Build a complete frontend for a multi-workload cloud platform that manages Virtual Machines, Serverless Functions, and Containers.

**Tech Stack:** Next.js 15 (App Router), React, TypeScript, TailwindCSS, shadcn/ui, Lucide icons

---

## Platform Overview

**Three Workload Types:**
1. **Virtual Machines** - Firecracker-based lightweight VMs
2. **Serverless Functions** - Lambda-like functions running in isolated MicroVMs
3. **Containers** - Docker container hosting

**Backend:** REST API (Rust + PostgreSQL) - all endpoints listed below
**Frontend:** Build from scratch using modern Next.js best practices

---

## Pages & Routes

### 1. Dashboard `/dashboard`

**Purpose:** Unified overview of all workloads (VMs, Functions, Containers)

**Stats Section:**
- Total VMs (running/total capacity)
- Total Functions (with 24h invocation count)
- Total Containers (running count)
- Total Hosts (available host count)

**All Resources Section:**
- Unified table/list showing VMs + Functions + Containers together
- Each row shows:
  - Name
  - Type (VM/Function/Container badge)
  - State (running/stopped/paused/idle/error)
  - Key metrics (CPU%, Memory%, Last invoked, etc.)
  - Quick actions (start/stop/delete)

**Features:**
- Search across all resources by name
- Filter by type (VM/Function/Container)
- Filter by state
- Quick create buttons (+ New VM, + Deploy Function, + Run Container)
- Real-time updates (states change live via WebSocket)

---

### 2. Virtual Machines

#### 2.1. VM List `/vms`

**Data per VM:**
- Name
- State (running/stopped/paused)
- CPU usage %
- Memory usage %
- Guest IP address
- Host name
- Created date
- Drive count
- NIC count

**Features:**
- Search by name
- Filter by state (running/stopped/paused)
- Sort by name, state, CPU, memory, created date
- Bulk select (multi-select for batch operations)
- Quick actions: Start, Stop, Pause, Resume, Delete
- [+ Create VM] button

#### 2.2. VM Detail `/vms/[id]`

**7 Tabs:**

**Overview Tab:**
- Current state badge
- Action buttons: Start, Stop, Pause, Resume, Delete, Send Ctrl-Alt-Del
- Info cards: vCPU count, Memory size, Host, Guest IP
- Resource usage: Current CPU %, Memory %
- Created/Updated timestamps

**Config Tab:**
- Machine config: vCPU, Memory, CPU template, SMT enabled, Track dirty pages
- Boot source: Kernel path, Rootfs path, Initrd path, Boot args
- All config values displayed clearly

**Storage Tab:**
- List of attached drives (table)
- Each drive: Drive ID, Path, Root device?, Read-only?, Rate limiter
- [+ Add Drive] button
- Actions per drive: Edit, Delete

**Network Tab:**
- List of NICs (table)
- Each NIC: Interface ID, Guest MAC, Host device, RX rate limiter, TX rate limiter
- [+ Add NIC] button
- Actions per NIC: Edit, Delete

**Terminal Tab:**
- Shell credentials display (username, password) with copy buttons
- Interactive terminal (xterm.js integration)
- WebSocket connection status indicator
- Fullscreen toggle

**Snapshots Tab:**
- List of snapshots (table)
- Each snapshot: Name, Type (Full/Diff), Created date
- [+ Create Snapshot] button
- Actions: Restore (creates new VM), Delete

**Metrics Tab:**
- Start/Stop monitoring button
- Real-time charts (update every 1s):
  - CPU usage % (line chart)
  - Memory usage % (line chart)
  - Network I/O (in/out, line chart)
  - Disk I/O (read/write, line chart)
- Latest values with badges/numbers
- Time window: Last 60 seconds

#### 2.3. Create VM `/vms/create`

**Multi-step wizard:**

**Step 1: Basic Info**
- Name (required)
- Description (optional)
- Environment (optional)
- Owner (optional)

**Step 2: Credentials**
- Username (default: root)
- Password (required)

**Step 3: Machine Config**
- vCPU count (slider/input: 1-32)
- Memory MiB (slider/input: 128-32768)
- CPU template (dropdown, optional)
- SMT enabled (checkbox)
- Track dirty pages (checkbox)

**Step 4: Boot Source**
- Kernel image (dropdown from registry or manual path)
- Rootfs image (dropdown from registry or manual path)
- Initrd path (optional)
- Boot args (optional text input)

**Step 5: Network**
- Enable networking (checkbox)
- Host device name (input)
- Guest MAC address (auto-generate button + manual input)

**Step 6: Review**
- Display all configured values
- Edit buttons to go back to each step
- [Create VM] button

---

### 3. Serverless Functions

#### 3.1. Functions List `/functions`

**Data per Function:**
- Name
- Runtime (Node.js 20.x, Python 3.11, Go 1.21, etc.)
- Status (idle/executing/error)
- Last invoked (timestamp, relative time)
- 24h invocations count
- Average duration (ms)
- Memory configured (MB)
- Timeout configured (seconds)

**Features:**
- Search by name
- Filter by runtime (Node.js/Python/Go/Rust)
- Filter by status
- Sort by name, last invoked, invocation count, avg duration
- Quick actions per function: Edit, Invoke (modal), Logs, Delete
- [+ New Function] button

#### 3.2. Function Editor `/functions/[id]` or `/functions/new`

**Layout: 3-panel IDE**

**Left Panel: Files**
- File tree (multi-file support)
- Quick files: index.js, utils.js, package.json, event.json
- Add/remove files

**Center Panel: Code Editor**
- Monaco Editor integration
- Syntax highlighting based on runtime
- Auto-completion
- File tabs if multiple files

**Right Panel: Test & Config**
- Test Event editor (JSON)
- Runtime selector: Node.js, Python, Go, Rust
- [Run Test] button
- Configuration panel (collapsible):
  - Function name
  - Handler (e.g., "index.handler")
  - Memory (slider: 128MB - 3GB)
  - Timeout (slider: 1s - 900s)
  - Environment variables (key-value editor)
  - Tags/labels

**Bottom Panel: Execution Results (collapsible)**
- Status (success/error/timeout badge)
- Duration (ms)
- Memory used (MB)
- Response (JSON formatted)
- Logs (console output)
- Request ID

**Top Actions:**
- [Save Draft] button
- [Deploy] button (save + make live)
- [Test] button
- Version selector (dropdown for version history)

#### 3.3. Function Logs `/functions/[id]/logs`

**Invocation logs viewer:**

**Data per invocation:**
- Timestamp
- Status badge (success/error/timeout)
- Duration (ms)
- Memory used (MB)
- Request ID (copyable)
- Event (collapsible JSON)
- Response (collapsible JSON)
- Logs (collapsible, multi-line)
- Error message (if failed)

**Features:**
- Real-time log streaming (WebSocket)
- Search logs content
- Filter by status (all/success/error/timeout)
- Time range selector (1h, 6h, 24h, 7d, custom)
- Auto-scroll toggle
- Export logs (CSV, JSON download)
- Metrics summary: Total invocations, Success rate %, Avg duration, p50/p95/p99 latency

---

### 4. Containers

#### 4.1. Containers List `/containers`

**Data per Container:**
- Name
- Image (e.g., "postgres:15", "nginx:latest")
- Status (running/stopped/restarting/error)
- Uptime (duration since started)
- CPU usage %
- Memory usage (used/limit)
- Network I/O (in/out rates)
- Port mappings (e.g., "5432→5432, 8080→80")

**Features:**
- Search by name
- Filter by status
- Filter by image name
- Sort by name, status, CPU, memory, uptime
- Quick actions: Logs, Shell, Stop, Restart, Delete
- [+ Deploy Container] button

#### 4.2. Container Detail `/containers/[id]`

**6 Tabs:**

**Overview Tab:**
- Status badge and uptime
- Action buttons: Start, Stop, Restart, Delete
- Image name and version
- Resource usage cards: CPU %, Memory (used/limit), Network I/O
- Port mappings list
- Environment variables (count, expandable list)
- Volumes (list)
- Container ID (copyable)

**Logs Tab:**
- Real-time log streaming (WebSocket)
- Search logs
- Filter by log level (if structured logs)
- Auto-scroll toggle (on by default)
- [Download Logs] button
- [Clear] button

**Shell Tab:**
- Interactive terminal (xterm.js, like VM terminal)
- WebSocket connection status
- Execute commands inside container
- Fullscreen toggle

**Stats Tab:**
- Time range selector (1h, 6h, 24h, 7d)
- CPU usage chart (line chart over time)
- Memory usage chart (line chart over time)
- Network I/O chart (in/out over time)
- Current values displayed

**Config Tab:**
- Image and tag
- Command and args
- Working directory
- Environment variables (table)
- Volumes/mounts (table)
- Port mappings (table)
- Restart policy
- Resource limits (CPU cores, Memory)
- Network mode
- Health check config

**Events Tab:**
- Container lifecycle events (table)
- Each event: Timestamp, Event type, Description
- Events: Created, Started, Stopped, Restarted, Killed, Error, Health check failed

#### 4.3. Deploy Container `/containers/new`

**Form sections:**

**Basic Configuration:**
- Container name (required)
- Image (required, text input with autocomplete for popular images)
- Quick image suggestions: postgres, nginx, redis, mongo, mysql

**Resources:**
- CPU limit (optional, slider: 0.1-16 cores)
- Memory limit (optional, slider: 64MB-32GB)

**Networking:**
- Port mappings (dynamic list):
  - Host port (input)
  - Container port (input)
  - Protocol (dropdown: TCP/UDP)
  - [+ Add Port] button
  - [Remove] button per mapping

**Environment Variables:**
- Dynamic key-value list
- Key (input)
- Value (input, can be masked for secrets)
- [+ Add Variable] button
- [Remove] button per variable

**Volumes:**
- Dynamic volume list
- Host path (input)
- Container path (input)
- Named volume toggle (use volume name instead of path)
- [+ Add Volume] button
- [Remove] button per volume

**Advanced Settings (collapsible/expandable):**
- Command override (text input)
- Args override (text input)
- Working directory (text input)
- Restart policy (dropdown: no, always, on-failure, unless-stopped)
- Network mode (dropdown: bridge, host, none)
- Health check:
  - Command (input)
  - Interval (input, seconds)
  - Timeout (input, seconds)
  - Retries (input, number)

**Actions:**
- [Cancel] button
- [Deploy] button (creates and starts container)

---

### 5. Image Registry `/registry`

**Purpose:** Manage kernel images, rootfs images, and volumes

**Data per Image:**
- Name
- Type badge (Kernel/Rootfs/Volume)
- Size (GB, formatted)
- Project/Category (optional tag)
- Path or URL
- Created date
- Usage count (how many VMs use this image)

**Features:**
- Search by name
- Filter by type (All/Kernel/Rootfs/Volume)
- Sort by name, size, created date, usage
- Actions per image: View Details, Clone, Download, Delete
- [+ Import Image] button
- [+ Create Volume] button

**Import Image Modal:**
- Source type: File path or URL
- File path (input)
- URL (input)
- Image type (dropdown: Kernel/Rootfs)
- Name (optional, auto-detect from filename)
- Project/Category (optional)

**Create Volume Modal:**
- Name (required)
- Size (input with unit: MB/GB)
- Project/Category (optional)

**Image Detail Modal:**
- Full metadata
- Path/URL
- Size (bytes, formatted)
- Checksum/hash
- Created date
- Used by (list of VM names)
- Tags/labels

---

### 6. Templates `/templates`

**Purpose:** Save and deploy VM configurations as templates

**Data per Template:**
- Name
- Description
- vCPU count
- Memory size
- Kernel image
- Rootfs image
- Created date
- Usage count (how many VMs created from this)

**Features:**
- Search by name
- Sort by name, created date, usage
- Actions per template: Deploy (instantiate), Edit, Clone, Delete
- [+ Create Template] button

**Create/Edit Template Modal:**
- Template name (required)
- Description (optional)
- Machine config: vCPU, Memory, CPU template
- Boot source: Kernel image, Rootfs image
- Network config: Enable networking, MAC address
- [Save Template] button

**Deploy from Template:**
- Quick form:
  - VM name (required)
  - Override settings (optional, expandable)
- [Deploy] button creates VM from template

---

### 7. Settings `/settings`

**Sections:**

**API Configuration:**
- API Endpoint URL (display only)
- WebSocket URL (display only)
- Authentication token (display/generate/revoke)

**User Preferences:**
- Theme selector (Dark/Light/Auto)
- Timezone (dropdown)
- Date format (dropdown: ISO/US/EU)
- Language (dropdown: EN/etc.)

**System Information:**
- Manager version
- Database status (connected/disconnected)
- Total hosts count
- Storage usage (used/total)

**Notifications (Future):**
- Email notifications toggle
- Webhook URL for alerts

---

## Navigation

**Global Navigation (Sidebar or Top Nav):**
- Dashboard
- Virtual Machines
- Functions
- Containers
- Registry
- Templates
- Settings

**User Menu (Top Right):**
- Theme toggle
- Settings
- Documentation link
- Logout (future)

---

## Backend API Endpoints

### VMs
```
POST   /v1/vms                        Create VM
GET    /v1/vms                        List all VMs
GET    /v1/vms/{id}                   Get VM details
DELETE /v1/vms/{id}                   Delete VM
POST   /v1/vms/{id}/start             Start VM
POST   /v1/vms/{id}/stop              Stop VM
POST   /v1/vms/{id}/pause             Pause VM
POST   /v1/vms/{id}/resume            Resume VM
POST   /v1/vms/{id}/ctrl-alt-del      Send Ctrl-Alt-Del
GET    /v1/vms/{id}/drives            List drives
POST   /v1/vms/{id}/drives            Add drive
PATCH  /v1/vms/{id}/drives/{drive_id} Update drive
DELETE /v1/vms/{id}/drives/{drive_id} Remove drive
GET    /v1/vms/{id}/nics              List NICs
POST   /v1/vms/{id}/nics              Add NIC
PATCH  /v1/vms/{id}/nics/{nic_id}     Update NIC
DELETE /v1/vms/{id}/nics/{nic_id}     Remove NIC
GET    /v1/vms/{id}/snapshots         List snapshots
POST   /v1/vms/{id}/snapshots         Create snapshot
POST   /v1/snapshots/{id}/instantiate Restore snapshot (new VM)
DELETE /v1/snapshots/{id}             Delete snapshot
GET    /v1/vms/{id}/shell             Get shell credentials
WS     /v1/vms/{id}/shell/ws          WebSocket terminal
WS     /v1/vms/{id}/metrics/ws        WebSocket metrics stream
```

### Functions
```
POST   /v1/functions                  Create function
GET    /v1/functions                  List all functions
GET    /v1/functions/{id}             Get function details
PUT    /v1/functions/{id}             Update function
DELETE /v1/functions/{id}             Delete function
POST   /v1/functions/{id}/invoke      Execute function
GET    /v1/functions/{id}/logs        Get execution logs
WS     /v1/functions/{id}/logs/ws     WebSocket log stream
```

### Containers
```
POST   /v1/containers                 Deploy container
GET    /v1/containers                 List all containers
GET    /v1/containers/{id}            Get container details
DELETE /v1/containers/{id}            Delete container
POST   /v1/containers/{id}/start      Start container
POST   /v1/containers/{id}/stop       Stop container
POST   /v1/containers/{id}/restart    Restart container
GET    /v1/containers/{id}/logs       Get logs
WS     /v1/containers/{id}/logs/ws    WebSocket log stream
WS     /v1/containers/{id}/shell/ws   WebSocket shell
```

### Images
```
GET    /v1/images                     List all images
POST   /v1/images                     Import/create image
GET    /v1/images/{id}                Get image details
DELETE /v1/images/{id}                Delete image
```

### Templates
```
POST   /v1/templates                  Create template
GET    /v1/templates                  List all templates
GET    /v1/templates/{id}             Get template details
PUT    /v1/templates/{id}             Update template
DELETE /v1/templates/{id}             Delete template
POST   /v1/templates/{id}/instantiate Deploy from template
```

---

## Data Models (TypeScript)

```typescript
// VM
interface VM {
  id: string;
  name: string;
  state: 'running' | 'stopped' | 'paused';
  host_id: string;
  host_addr: string;
  vcpu: number;
  mem_mib: number;
  kernel_path: string;
  rootfs_path: string;
  guest_ip?: string;
  created_at: string;
  updated_at: string;
}

// Function
interface Function {
  id: string;
  name: string;
  runtime: 'node' | 'python' | 'go' | 'rust';
  code: string;
  handler: string;
  timeout_seconds: number;
  memory_mb: number;
  env_vars?: Record<string, string>;
  created_at: string;
  updated_at: string;
  last_invoked_at?: string;
}

// Function Invocation Log
interface FunctionInvocation {
  id: string;
  function_id: string;
  status: 'success' | 'error' | 'timeout';
  duration_ms: number;
  memory_used_mb: number;
  request_id: string;
  event: any;
  response?: any;
  logs: string[];
  error?: string;
  invoked_at: string;
}

// Container
interface Container {
  id: string;
  name: string;
  image: string;
  status: 'running' | 'stopped' | 'restarting' | 'error';
  uptime_seconds?: number;
  cpu_percent?: number;
  memory_used_mb?: number;
  memory_limit_mb?: number;
  port_mappings: PortMapping[];
  env_vars?: Record<string, string>;
  volumes?: VolumeMount[];
  command?: string;
  args?: string[];
  restart_policy?: string;
  created_at: string;
  started_at?: string;
}

interface PortMapping {
  host: number;
  container: number;
  protocol: 'tcp' | 'udp';
}

interface VolumeMount {
  host: string;
  container: string;
}

// Image
interface Image {
  id: string;
  name: string;
  kind: 'kernel' | 'rootfs';
  size_bytes?: number;
  project?: string;
  path?: string;
  url?: string;
  created_at: string;
}

// Snapshot
interface Snapshot {
  id: string;
  vm_id: string;
  snapshot_type: 'Full' | 'Diff';
  snapshot_path: string;
  mem_file_path?: string;
  created_at: string;
}

// Template
interface Template {
  id: string;
  name: string;
  description?: string;
  vcpu: number;
  mem_mib: number;
  kernel_path: string;
  rootfs_path: string;
  created_at: string;
}

// Drive
interface VmDrive {
  drive_id: string;
  path_on_host: string;
  is_root_device: boolean;
  is_read_only: boolean;
  rate_limiter?: RateLimiter;
}

// NIC
interface VmNic {
  iface_id: string;
  guest_mac: string;
  host_dev_name: string;
  rx_rate_limiter?: RateLimiter;
  tx_rate_limiter?: RateLimiter;
}

interface RateLimiter {
  bandwidth?: {
    size: number;
    refill_time: number;
  };
  ops?: {
    size: number;
    refill_time: number;
  };
}

// Real-time Metrics
interface VmMetrics {
  cpu_usage_percent: number;
  memory_usage_percent: number;
  memory_used_kb: number;
  memory_total_kb: number;
  network_in_bytes: number;
  network_out_bytes: number;
  disk_read_bytes: number;
  disk_write_bytes: number;
}
```

---

## Real-time Features (WebSocket)

**WebSocket connections needed:**

1. **VM Metrics** - `/v1/vms/{id}/metrics/ws`
   - Receive metrics every 1 second
   - Update charts in real-time

2. **VM Terminal** - `/v1/vms/{id}/shell/ws`
   - Bidirectional terminal I/O
   - Use xterm.js for display

3. **Function Logs** - `/v1/functions/{id}/logs/ws`
   - Stream new invocation logs
   - Append to logs list in real-time

4. **Container Logs** - `/v1/containers/{id}/logs/ws`
   - Stream container stdout/stderr
   - Auto-scroll option

5. **Container Shell** - `/v1/containers/{id}/shell/ws`
   - Interactive shell like VM terminal

---

## UI/UX Requirements

**Must Have:**
- Responsive design (mobile + desktop)
- Dark mode (primary) + Light mode support
- Keyboard shortcuts for common actions
- Loading states (skeletons, spinners)
- Error states with helpful messages
- Empty states with CTAs
- Confirmation dialogs for destructive actions
- Toast notifications for success/error feedback
- Real-time updates (WebSocket integration)
- Accessible (ARIA labels, keyboard navigation)

**Data Display:**
- Tables with sorting and filtering
- Card views as alternative to tables
- Pagination or infinite scroll for long lists
- Search with debouncing
- Multi-select for bulk operations
- Copy-to-clipboard buttons for IDs, credentials, etc.

**Forms:**
- Inline validation
- Clear error messages
- Default values where applicable
- Auto-save drafts (functions)
- Wizard/stepper for multi-step forms
- Preview/review before submit

**Charts:**
- Real-time line charts for metrics
- Sparklines for quick trends
- Gauge charts for percentages
- Time range selectors
- Responsive sizing

---

## File Structure

```
apps/frontend/
├── app/
│   ├── (dash)/                     # Dashboard layout
│   │   ├── layout.tsx              # Shared layout with nav
│   │   ├── dashboard/
│   │   │   └── page.tsx            # Dashboard page
│   │   ├── vms/
│   │   │   ├── page.tsx            # VM list
│   │   │   ├── [id]/page.tsx       # VM detail
│   │   │   └── create/page.tsx     # Create VM wizard
│   │   ├── functions/
│   │   │   ├── page.tsx            # Functions list
│   │   │   ├── [id]/page.tsx       # Function editor
│   │   │   └── [id]/logs/page.tsx  # Function logs
│   │   ├── containers/
│   │   │   ├── page.tsx            # Containers list
│   │   │   ├── [id]/page.tsx       # Container detail
│   │   │   └── new/page.tsx        # Deploy container
│   │   ├── registry/
│   │   │   └── page.tsx            # Registry browser
│   │   ├── templates/
│   │   │   └── page.tsx            # Templates list
│   │   └── settings/
│   │       └── page.tsx            # Settings
│   ├── api/                        # API routes (if needed)
│   └── layout.tsx                  # Root layout
├── components/
│   ├── ui/                         # shadcn/ui components
│   ├── vm/                         # VM-specific components
│   ├── function/                   # Function-specific components
│   ├── container/                  # Container-specific components
│   ├── shared/                     # Shared components
│   │   ├── ResourceCard.tsx
│   │   ├── StatusBadge.tsx
│   │   ├── MetricsChart.tsx
│   │   ├── Terminal.tsx
│   │   ├── LogViewer.tsx
│   │   └── ...
│   └── layout/
│       ├── Sidebar.tsx
│       ├── Topbar.tsx
│       └── ...
├── lib/
│   ├── api/
│   │   ├── client.ts               # Base HTTP client
│   │   ├── vms.ts                  # VM API calls
│   │   ├── functions.ts            # Function API calls
│   │   ├── containers.ts           # Container API calls
│   │   ├── images.ts               # Image API calls
│   │   └── templates.ts            # Template API calls
│   ├── hooks/
│   │   ├── useVMs.ts               # React Query hooks for VMs
│   │   ├── useFunctions.ts         # React Query hooks for Functions
│   │   ├── useContainers.ts        # React Query hooks for Containers
│   │   └── useWebSocket.ts         # WebSocket hook
│   ├── utils/
│   │   ├── format.ts               # Formatters (bytes, date, etc.)
│   │   └── validation.ts           # Form validators
│   └── types/
│       └── index.ts                # TypeScript types
└── styles/
    └── globals.css                 # Global styles
```

---

## Integration Notes

**Monaco Editor:**
- Use `@monaco-editor/react` package
- Language modes: javascript, python, go, rust, json
- Theme: Match app theme (dark/light)
- Features: Auto-complete, syntax highlighting

**xterm.js:**
- Use `xterm` and `xterm-addon-fit` packages
- WebSocket integration for terminal I/O
- Fit terminal to container size
- Copy/paste support

**Charts:**
- Use Chart.js with `react-chartjs-2` or Recharts
- Real-time data updates
- Responsive sizing
- Time series for metrics

**React Query:**
- Use for all API data fetching
- Cache management
- Optimistic updates
- Mutation handling

**WebSocket:**
- Custom hook for WebSocket connections
- Auto-reconnect on disconnect
- Message parsing and routing
- Connection state management

---

## Priorities

Build in this order:

**Phase 1: Core Structure**
1. Layout and navigation
2. Dashboard page
3. VM list page
4. Function list page
5. Container list page

**Phase 2: Detail Pages**
6. VM detail with tabs
7. Function editor
8. Container detail with tabs

**Phase 3: Actions**
9. Create VM wizard
10. Function creation
11. Container deployment
12. Log viewers

**Phase 4: Additional Features**
13. Registry browser
14. Templates management
15. Settings page

---

**Build the entire frontend from scratch. Create all pages, components, API clients, and hooks. Focus on clean code, type safety, and great UX.**
