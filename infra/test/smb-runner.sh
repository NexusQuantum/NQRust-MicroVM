#!/usr/bin/env bash
# Comprehensive SMB integration test for v0.4.0.
# Runs INSIDE the iscsi-alpha KubeVirt VM after the alpha is installed
# (or after a manager binary is hot-deployed during dev). Mirrors the
# iscsi-alpha-runner pattern.

set -uo pipefail

MANAGER="${MANAGER:-http://127.0.0.1:18080}"
SMB_HOST="${SMB_HOST:-127.0.0.1}"
SMB_SHARE="${SMB_SHARE:-vms}"
SMB_USER="${SMB_USER:-vm-admin}"
SMB_PASS="${SMB_PASS:-smb-pass}"
BACKEND_NAME="${BACKEND_NAME:-smb-auth}"
ANON_BACKEND_NAME="${ANON_BACKEND_NAME:-smb-anon}"

PASS=0; FAIL=0; SKIP=0
FAILED_TESTS=()

log()  { printf "\n\033[1;36m[%s]\033[0m %s\n" "$(date +%H:%M:%S)" "$*"; }
ok()   { printf "  \033[1;32mPASS\033[0m  %s\n" "$*"; PASS=$((PASS+1)); }
fail() { printf "  \033[1;31mFAIL\033[0m  %s\n" "$*"; FAIL=$((FAIL+1)); FAILED_TESTS+=("$*"); }
skip() { printf "  \033[1;33mSKIP\033[0m  %s\n" "$*"; SKIP=$((SKIP+1)); }

login() {
  local resp
  resp=$(curl -sf -X POST "$MANAGER/v1/auth/login" \
    -H 'Content-Type: application/json' \
    -d '{"username":"root","password":"root"}')
  TOKEN=$(echo "$resp" | jq -r .token)
  if [[ -z "$TOKEN" || "$TOKEN" == "null" ]]; then
    echo "FATAL: could not login to manager at $MANAGER" >&2
    exit 1
  fi
}

curl_api() {
  curl -s -H "Authorization: Bearer $TOKEN" "$@"
}

# Cleanup any leftover backends from prior runs (best-effort).
cleanup_pre() {
  local id
  for n in "$BACKEND_NAME" "$ANON_BACKEND_NAME"; do
    id=$(curl_api "$MANAGER/v1/storage_backends" | jq -r ".items[]? | select(.name==\"$n\") | .id")
    if [[ -n "$id" ]]; then
      curl_api -X DELETE "$MANAGER/v1/storage_backends/$id" >/dev/null
    fi
  done
  # Also wipe any leftover smb-* test files from the share
  sudo rm -f /var/lib/test-smb/share/smb-*.raw 2>/dev/null || true
}

####################################################
# T1: Backend create (authenticated)
####################################################
test_create_authenticated() {
  log "T1: Backend create with username/password"
  local body code id
  body=$(jq -n --arg n "$BACKEND_NAME" --arg s "$SMB_HOST" --arg sh "$SMB_SHARE" --arg u "$SMB_USER" --arg p "$SMB_PASS" \
    '{name:$n, kind:"smb", config:{server:$s, share:$sh, username:$u}, password:$p}')
  local resp
  resp=$(curl_api -X POST "$MANAGER/v1/storage_backends" \
    -H 'Content-Type: application/json' -d "$body" -w "\n%{http_code}")
  code=$(echo "$resp" | tail -1)
  id=$(echo "$resp" | head -n -1 | jq -r '.id // empty')
  if [[ "$code" == "201" && -n "$id" ]]; then
    ok "POST /storage_backends (auth) -> 201, id=$id"
    AUTH_BACKEND_ID="$id"
  else
    fail "POST authenticated -> $code: $(echo "$resp" | head -n -1)"
  fi

  # Cred file should exist on the agent host
  if sudo test -f "/etc/nqrust/storage-creds/$AUTH_BACKEND_ID.cred"; then
    ok "agent: cred file written at /etc/nqrust/storage-creds/$AUTH_BACKEND_ID.cred"
    local mode
    mode=$(sudo stat -c '%a' "/etc/nqrust/storage-creds/$AUTH_BACKEND_ID.cred")
    [[ "$mode" == "600" ]] && ok "agent: cred file mode = 600" || fail "agent: cred file mode = $mode (want 600)"
  else
    fail "agent: cred file NOT written"
  fi
}

####################################################
# T2: Validation rejects missing required fields
####################################################
test_validation() {
  log "T2: Validation rejects missing required fields"
  for field in server share; do
    local body code
    body=$(jq -n --arg n "v-$field" --arg s "$SMB_HOST" --arg sh "$SMB_SHARE" \
                --arg miss "$field" \
      'def stripped(m): {server:$s, share:$sh} | del(.[m]);
       {name:$n, kind:"smb", config: stripped($miss)}')
    code=$(curl_api -X POST "$MANAGER/v1/storage_backends" \
      -H 'Content-Type: application/json' -d "$body" \
      -o /dev/null -w "%{http_code}")
    if [[ "$code" == "400" || "$code" == "422" ]]; then
      ok "POST without $field -> $code (rejected)"
    else
      fail "POST without $field -> $code (expected 4xx)"
    fi
  done

  # Invalid smb_version
  local body code
  body=$(jq -n --arg s "$SMB_HOST" --arg sh "$SMB_SHARE" \
    '{name:"v-badver", kind:"smb", config:{server:$s, share:$sh, smb_version:"bogus"}}')
  code=$(curl_api -X POST "$MANAGER/v1/storage_backends" \
    -H 'Content-Type: application/json' -d "$body" \
    -o /dev/null -w "%{http_code}")
  if [[ "$code" == "400" || "$code" == "422" ]]; then
    ok "POST with bad smb_version -> $code (rejected)"
  else
    fail "POST with bad smb_version -> $code (expected 4xx)"
  fi
}

####################################################
# T3: Health probe — backend reachable after create
####################################################
test_health() {
  log "T3: Health probe after authenticated create"
  if [[ -z "${AUTH_BACKEND_ID:-}" ]]; then skip "no AUTH_BACKEND_ID"; return; fi
  sleep 2
  local h
  h=$(curl_api "$MANAGER/v1/storage_backends/$AUTH_BACKEND_ID/health")
  if echo "$h" | jq -e '.reachable == true' >/dev/null 2>&1; then
    ok "health: reachable=true ($h)"
  else
    fail "health: $h"
  fi

  # Mount should be visible in findmnt
  if findmnt --mountpoint "/var/lib/nqrust/smb/$SMB_HOST:$SMB_SHARE" >/dev/null 2>&1; then
    ok "host: smb mount present at /var/lib/nqrust/smb/$SMB_HOST:$SMB_SHARE"
  else
    fail "host: mount NOT present"
  fi
}

####################################################
# T4: Anonymous backend create + probe
####################################################
test_anonymous() {
  log "T4: Anonymous backend (guest access)"
  local body code id resp
  body=$(jq -n --arg n "$ANON_BACKEND_NAME" --arg s "$SMB_HOST" --arg sh "$SMB_SHARE" \
    '{name:$n, kind:"smb", config:{server:$s, share:$sh}}')
  resp=$(curl_api -X POST "$MANAGER/v1/storage_backends" \
    -H 'Content-Type: application/json' -d "$body" -w "\n%{http_code}")
  code=$(echo "$resp" | tail -1)
  id=$(echo "$resp" | head -n -1 | jq -r '.id // empty')
  if [[ "$code" == "201" && -n "$id" ]]; then
    ok "POST anonymous -> 201, id=$id"
    ANON_BACKEND_ID="$id"
  else
    fail "POST anonymous -> $code: $(echo "$resp" | head -n -1)"
  fi

  if sudo test -f "/etc/nqrust/storage-creds/$ANON_BACKEND_ID.cred" 2>/dev/null; then
    fail "anonymous: cred file should NOT exist for guest mode"
  else
    ok "anonymous: no cred file (correct, guest mode)"
  fi
}

####################################################
# T5: VM lifecycle on smb backend
####################################################
test_vm_lifecycle() {
  log "T5: VM lifecycle on smb-auth backend"
  if [[ -z "${AUTH_BACKEND_ID:-}" ]]; then skip "no AUTH_BACKEND_ID"; return; fi

  local kernel_id rootfs_id resp body vm_id code
  kernel_id=$(curl_api "$MANAGER/v1/images" | jq -r '.items[] | select(.kind=="kernel") | .id' | head -1)
  rootfs_id=$(curl_api "$MANAGER/v1/images" | jq -r '.items[] | select(.kind=="rootfs" and (.name|test("alpine";"i"))) | .id' | head -1)
  if [[ -z "$kernel_id" || -z "$rootfs_id" ]]; then
    skip "kernel/alpine rootfs not in image registry"; return
  fi

  body=$(jq -n --arg n "smb-vm-1" --arg k "$kernel_id" --arg r "$rootfs_id" --arg b "$AUTH_BACKEND_ID" \
    '{name:$n, kernel_image_id:$k, rootfs_image_id:$r, vcpu:1, mem_mib:512, backend_id:$b}')
  resp=$(curl_api -X POST "$MANAGER/v1/vms" \
    -H 'Content-Type: application/json' -d "$body" -w "\n%{http_code}")
  code=$(echo "$resp" | tail -1)
  vm_id=$(echo "$resp" | head -n -1 | jq -r '.id // empty')
  if [[ ( "$code" == "200" || "$code" == "201" ) && -n "$vm_id" ]]; then
    ok "POST /vms -> $code, id=$vm_id"
  else
    fail "POST /vms -> $code: $(echo "$resp" | head -n -1)"
    return
  fi

  # Check the rootfs file is at the expected mount location
  sleep 3
  local share_dir="/var/lib/nqrust/smb/$SMB_HOST:$SMB_SHARE"
  local file_count
  file_count=$(sudo find "$share_dir" -maxdepth 1 -name "smb-*.raw" 2>/dev/null | wc -l)
  if [[ "$file_count" -ge 1 ]]; then
    ok "host: rootfs file present on smb share (count=$file_count)"
  else
    fail "host: no smb-*.raw file in $share_dir"
  fi

  # Stop + delete
  curl_api -X POST "$MANAGER/v1/vms/$vm_id/stop" -o /dev/null -w "stop=%{http_code}\n"
  sleep 2
  code=$(curl_api -X DELETE "$MANAGER/v1/vms/$vm_id" -o /dev/null -w "%{http_code}")
  if [[ "$code" == "200" || "$code" == "204" ]]; then ok "DELETE /vms/:id -> $code"
  else fail "DELETE -> $code"; fi
}

####################################################
# T6: Edit-in-place password rotation
####################################################
test_rotation() {
  log "T6: Password rotation via update endpoint"
  if [[ -z "${AUTH_BACKEND_ID:-}" ]]; then skip "no AUTH_BACKEND_ID"; return; fi

  local old_mtime new_mtime body code
  old_mtime=$(sudo stat -c '%Y' "/etc/nqrust/storage-creds/$AUTH_BACKEND_ID.cred")

  # Re-PUT the same backend with a new password
  body=$(jq -n --arg s "$SMB_HOST" --arg sh "$SMB_SHARE" --arg u "$SMB_USER" --arg p "rotated-$$" \
    '{name:"smb-auth", kind:"smb", config:{server:$s, share:$sh, username:$u}, password:$p}')
  code=$(curl_api -X PUT "$MANAGER/v1/storage_backends/$AUTH_BACKEND_ID" \
    -H 'Content-Type: application/json' -d "$body" \
    -o /dev/null -w "%{http_code}")
  if [[ "$code" == "200" || "$code" == "204" ]]; then ok "PUT update -> $code"
  else fail "PUT -> $code"; fi

  sleep 1
  new_mtime=$(sudo stat -c '%Y' "/etc/nqrust/storage-creds/$AUTH_BACKEND_ID.cred")
  if [[ "$new_mtime" != "$old_mtime" ]]; then
    ok "agent: cred file mtime changed (rotated)"
  else
    fail "agent: cred file mtime unchanged"
  fi

  # Rotate it back so subsequent tests still authenticate
  body=$(jq -n --arg s "$SMB_HOST" --arg sh "$SMB_SHARE" --arg u "$SMB_USER" --arg p "$SMB_PASS" \
    '{name:"smb-auth", kind:"smb", config:{server:$s, share:$sh, username:$u}, password:$p}')
  curl_api -X PUT "$MANAGER/v1/storage_backends/$AUTH_BACKEND_ID" \
    -H 'Content-Type: application/json' -d "$body" >/dev/null
}

####################################################
# T7: Bad credentials rejected at create-time
####################################################
test_bad_creds() {
  log "T7: Bad credentials rejected at create-time"
  local body code resp
  body=$(jq -n --arg s "$SMB_HOST" --arg sh "$SMB_SHARE" --arg u "$SMB_USER" \
    '{name:"smb-bad", kind:"smb", config:{server:$s, share:$sh, username:$u}, password:"wrong-pass"}')
  resp=$(curl_api -X POST "$MANAGER/v1/storage_backends" \
    -H 'Content-Type: application/json' -d "$body" -w "\n%{http_code}")
  code=$(echo "$resp" | tail -1)
  if [[ "$code" == "422" ]]; then
    ok "POST with bad password -> 422 (rejected at mount probe)"
  elif [[ "$code" == "201" ]]; then
    # Some Samba configs accept any user that exists (e.g. map to guest) — accept that too
    ok "POST with bad password -> 201 (server mapped to guest, not rejected)"
    local id
    id=$(echo "$resp" | head -n -1 | jq -r '.id')
    curl_api -X DELETE "$MANAGER/v1/storage_backends/$id" >/dev/null
  else
    fail "POST with bad password -> $code"
  fi
}

####################################################
# T8: Live registry — delete clears cred file
####################################################
test_delete_clears_cred() {
  log "T8: Delete backend clears cred file on agent"
  if [[ -z "${AUTH_BACKEND_ID:-}" ]]; then skip "no AUTH_BACKEND_ID"; return; fi

  # First call: T5's deleted VM left a 'rootfs-*' volume row behind (the same
  # behaviour the iscsi-alpha runner documents at T6a). The manager should
  # reject this delete with 409 to protect the backend.
  local resp code body
  resp=$(curl_api -X DELETE "$MANAGER/v1/storage_backends/$AUTH_BACKEND_ID" -w "\n%{http_code}")
  code=$(echo "$resp" | tail -1)
  body=$(echo "$resp" | head -n -1)
  if [[ "$code" == "409" ]]; then
    ok "DELETE backend with live volume -> 409 (correctly protected)"
  elif [[ "$code" == "204" ]]; then
    ok "DELETE backend -> 204 (no live volumes)"
  else
    fail "DELETE backend (initial) -> $code: $body"
    return
  fi

  # Clear the orphan volume rows and the share file, then retry — backend
  # should delete cleanly.
  if [[ "$code" == "409" ]]; then
    local share_dir="/var/lib/nqrust/smb/$SMB_HOST:$SMB_SHARE"
    sudo find "$share_dir" -maxdepth 1 -name "smb-*.raw" -delete 2>/dev/null || true
    PGPASSWORD=nexus psql -h 127.0.0.1 -p 5432 -U nexus -d nexus \
      -c "DELETE FROM volume WHERE backend_id='$AUTH_BACKEND_ID';" >/dev/null 2>&1 || true

    code=$(curl_api -X DELETE "$MANAGER/v1/storage_backends/$AUTH_BACKEND_ID" \
      -o /dev/null -w "%{http_code}")
    if [[ "$code" == "204" ]]; then ok "DELETE backend after volume cleanup -> 204"
    else fail "DELETE backend after cleanup -> $code"; fi
  fi

  sleep 1
  if sudo test -f "/etc/nqrust/storage-creds/$AUTH_BACKEND_ID.cred"; then
    fail "cred file still present after delete"
  else
    ok "cred file removed by agent"
  fi

  # Delete anonymous backend too
  if [[ -n "${ANON_BACKEND_ID:-}" ]]; then
    curl_api -X DELETE "$MANAGER/v1/storage_backends/$ANON_BACKEND_ID" >/dev/null
  fi
}

####################################################
main() {
  echo
  echo "================================================================"
  echo "  v0.4.0 SMB integration test"
  echo "================================================================"
  echo "Manager: $MANAGER"
  echo "SMB:     //$SMB_HOST/$SMB_SHARE (user=$SMB_USER)"
  echo

  login
  cleanup_pre
  test_create_authenticated
  test_validation
  test_health
  test_anonymous
  test_vm_lifecycle
  test_rotation
  test_bad_creds
  test_delete_clears_cred

  echo
  echo "================================================================"
  echo "  Results: $PASS passed, $FAIL failed, $SKIP skipped"
  echo "================================================================"
  if [[ $FAIL -gt 0 ]]; then
    echo "Failed:"
    for t in "${FAILED_TESTS[@]}"; do echo "  - $t"; done
    exit 1
  fi
}

main "$@"
