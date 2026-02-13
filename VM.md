Here's a comprehensive review of the VM management system — what's solid and what can be improved:

What's already solid
Clean layered architecture (routes → service → repo)
Full Firecracker device support (drives, NICs, balloon, VSOCK, MMDS, entropy)
6-step creation wizard with zod validation
Shell access via xterm.js WebSocket
Snapshots (create/restore/delete)
Port forwarding with iptables
Role-based access control
Network + VXLAN auto-expansion on VM create
Areas for improvement
High Impact, Quick Wins
#	Issue	Where	Effort
1	No auto-refresh — VM state is a snapshot from page load, goes stale immediately	useVMs() / useVM() in queries.ts	Low
2	No VM rename — name is immutable after creation	Backend routes.rs + frontend overview	Low
3	No tag editor — VMs have tags[] but no UI to view/edit them	vm detail page, new component	Medium
4	Inconsistent delete rules — NIC delete disabled while running, but port forward delete works live	vm-network.tsx	Low
5	Edit drive dialog is read-only — button says "Edit" but nothing is editable	vm-storage.tsx	Low
6	Metrics tab empty — tab exists but MetricsChart isn't wired up	vm detail [id]/page.tsx	Medium
High Impact, Medium Effort
#	Issue	Where
7	No bulk operations — can't multi-select VMs to start/stop/delete	vm-table.tsx, new batch API
8	No server-side pagination — GET /v1/vms returns everything; UI paginates client-side	Backend list endpoint + frontend
9	Terminal disconnects on tab change — WebSocket drops, must reconnect manually	vm-terminal.tsx
10	No resource usage graphs — CPU/mem shown as single % number, no trend	MetricsChart component, WebSocket
11	Generic error responses — backend returns bare 500 with no detail for most failures	routes.rs error mapping
Nice to Have (Bigger Features)
#	Feature	Notes
12	VM cloning — create copy of existing VM	New endpoint + service logic
13	Live resize — change vCPU/memory without restart	Firecracker hot-plug API
14	Config export/import — export VM config as JSON	New endpoint
15	Snapshot scheduling — automatic periodic snapshots	Background task
16	Advanced search — filter by tags, host, IP, not just name	vm-table.tsx filter panel
Code Quality Issues
service.rs is ~2500 lines — could split by concern (lifecycle, devices, networking)
created_by_user_id accessed as (vm as any).created_by_user_id — missing from TypeScript Vm type
CreateNicReq missing network_id — type doesn't match what frontend sends
No optimistic updates — mutations wait for server round-trip before UI updates
Hardcoded paths (/srv/fc/vms, /srv/images) scattered through service code
What would you like to tackle first? I'd suggest starting with auto-refresh + VM rename + tag editing as quick high-impact wins, or if you want a bigger project, bulk operations + server-side pagination.