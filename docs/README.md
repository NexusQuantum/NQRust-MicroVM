# NexusRust — A1–A3 Quickstart


## 0) Infra
```bash
./scripts/dev-up.sh
```


## 1) Bridge (once per host)
```bash
sudo ./scripts/fc-bridge-setup.sh fcbr0 <uplink-iface>
```


## 2) Agent (KVM host)
```bash
export AGENT_BIND=127.0.0.1:9090
export FC_RUN_DIR=/srv/fc
export FC_BRIDGE=fcbr0
(cd apps/agent && cargo run)
```


## 3) Manager
```bash
export DATABASE_URL=postgres://nexus:nexus@localhost:5432/nexus
export MANAGER_BIND=127.0.0.1:8080
export AGENT_BASE=http://127.0.0.1:9090
(cd apps/manager && sqlx migrate run && cargo run)
```


## 4) Create a VM
```bash
curl -sS -X POST http://127.0.0.1:8080/v1/vms \
-H 'content-type: application/json' \
-d '{
"name":"demo",
"vcpu":1,
"mem_mib":256,
"kernel_path":"/path/to/hello-vmlinux.bin",
"rootfs_path":"/path/to/hello-rootfs.ext4"
}'
```


> Ensure kernel/rootfs exist and are readable. Firecracker must be in PATH.