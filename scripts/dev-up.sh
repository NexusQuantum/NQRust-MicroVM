#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if command -v docker compose >/dev/null 2>&1; then DC="docker compose"; else DC="docker-compose"; fi
$DC -f infra/docker-compose.dev.yml up -d


echo "Postgres up on 5432 (user=nexus pass=nexus db=nexus)"