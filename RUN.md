export the env

# Agent
cargo build -p agent
sudo -E env AGENT_BIND=127.0.0.1:19090 MANAGER_BASE=http://127.0.0.1:18080 FC_RUN_DIR=/srv/fc FC_BRIDGE=fcbr0 ./target/debug/agent

# Manager
cargo build -p manager
export DATABASE_URL=postgres://nexus:nexus@localhost:5432/nexus
MANAGER_RECONCILER_DISABLED=1 RUST_LOG=info ./target/debug/manager

# If migrations fail, reset migration 10:
# psql $DATABASE_URL -c "DELETE FROM _sqlx_migrations WHERE version = 10;"
# cd apps/manager && sqlx migrate run


curl -v -X POST http://127.0.0.1:18080/v1/vms \
  -H 'Content-Type: application/json' \
  -d '{"name":"vm-alpine-test","vcpu":1,"mem_mib":512,"kernel_image_id":"59e1c754-2210-4887-858c-f3c5de7d483b","rootfs_image_id":"4196a86f-95f4-4609-af23-138ec331b0dc"}'


Frontend (Next.js)
# Local development configuration
export NEXT_PUBLIC_API_BASE_URL=http://localhost:18081/v1
export NEXT_PUBLIC_WS_BASE_URL=ws://localhost:18081
export NEXT_PUBLIC_BRAND_PRESET=dark


cd apps/frontend
pnpm i
export NEXT_PUBLIC_API_BASE_URL=http://127.0.0.1:18080/v1
pnpm dev

Open http://localhost:3000/vms