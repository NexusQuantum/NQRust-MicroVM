# SMB (CIFS) storage backend — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `smb` storage backend (file protocol, parallel to `nfs`) so operators can configure SMB/CIFS shares from the UI like NFS. Anonymous + authenticated shares, agent-managed cred file (Proxmox-style), live mount probe + edit + rotation.

**Architecture:** Mirror `nfs` end-to-end — agent owns privileged `mount.cifs`, manager unprivileged, NEW kind `BackendKind::Smb`. New piece vs NFS is **credential delivery**: manager posts username+password to a new agent route that writes a 0600-mode cred file (`/etc/nqrust/storage-creds/<backend-id>.cred`); `mount.cifs` reads it via `-o credentials=<file>`. Anonymous shares mount with `-o guest`. Reference: `~/refs/pve-storage/src/PVE/Storage/CIFSPlugin.pm` (354 lines, same pattern we're porting).

**Tech Stack:** Rust (manager + agent), `mount.cifs` (cifs-utils), Axum, Next.js 15, Postgres. Test target: in-VM Samba 4 instance (`smbd`) inside the KubeVirt iscsi-alpha VM, file-backed share.

---

## Scope Check

- ✅ Authenticated SMB shares (username + password).
- ✅ Anonymous SMB shares (`-o guest`).
- ✅ SMB version select (enum: `default | 2.0 | 2.1 | 3 | 3.0 | 3.11`).
- ✅ Domain/workgroup field (optional, for AD).
- ✅ Subdir field (optional, mount only `<share>/<subdir>`).
- ✅ Freeform `options` field (advanced — comma-separated mount.cifs flags).
- ✅ Edit-in-place dialog supports password rotation (re-sends cred file to agent).
- ❌ **Kerberos auth** — operator can use `options: "sec=krb5"` but UI doesn't manage the keytab. Out of scope.
- ❌ **Per-VM file permission mapping** — uid/gid stays at 0/0 by default (raw rootfs files); operators can override via `options`.
- ❌ **Browse SMB share contents from UI** — could be a nice future feature.

---

## File Structure

### New files

- `crates/nexus-storage/src/types.rs` — extend (add `Smb` variant).
- `crates/nexus-types/src/lib.rs` — extend (mirror variant).
- `apps/ui/lib/types/index.ts` — extend (TS union).
- `apps/manager/migrations/0039_smb_backend_kind.sql` — kind CHECK constraint update.
- `apps/manager/src/features/storage/backends/smb.rs` — `SmbConfig`, `SmbLocator`, `SmbControlPlaneBackend`.
- `apps/agent/src/features/storage/smb.rs` — `SmbHostBackend`, mount helpers, cred-file write, host-side `HostBackend` impl.
- `apps/agent/src/features/storage/smb/routes.rs` (inline in smb.rs if small) — `/v1/storage/smb/*` HTTP routes.
- `docs/runbooks/smb-troubleshooting.md` — auth failures, version mismatches, common SAMBA quirks.

### Modified files

- `apps/manager/src/features/storage/registry.rs` — `build_backend` arm for Smb.
- `apps/manager/src/features/storage/config.rs` — `validate` arm: required `server`, `share`. `username`+`password` optional but together (XOR with guest).
- `apps/manager/src/features/storage_backends/health.rs` — health probe (TCP 445 + mount check).
- `apps/manager/src/features/storage_backends/routes.rs` — on `create` + `update`, send credentials to agent after upsert.
- `apps/manager/src/features/vms/service.rs` — no change (`host_path_for` trait default is fine — locator is `{server, share, file}`, returns `/var/lib/nqrust/smb/<key>/<file>`).
- `apps/agent/src/features/storage/mod.rs` (or wherever NFS nests) — mount `smb::router()` under `/v1/storage/smb`.
- `apps/agent/src/main.rs` — register `SmbHostBackend` when `mount.cifs` is on PATH.
- `scripts/install/lib/deps.sh` — add `cifs-utils` (Debian) / `cifs-utils` (RHEL).
- `apps/installer/src/installer/deps.rs` — same.
- `scripts/airgap/bundle-debs-ubuntu.sh` — same.
- `apps/ui/components/storage/backend-create-dialog.tsx` — `smb` field schema + new `type: "password"` + `type: "select"` Field shapes.
- `apps/ui/lib/types/index.ts` — `BackendKind` union add `"smb"`.
- `apps/ui/components/storage/backend-table.tsx` — KIND_LABEL entry.
- `CHANGELOG.md` — entry.

---

## Task Breakdown

### Task 1: `BackendKind::Smb` enum variant

**Files:**
- Modify: `crates/nexus-storage/src/types.rs`
- Modify: `crates/nexus-types/src/lib.rs`
- Modify: `apps/ui/lib/types/index.ts`
- Modify: `apps/manager/src/features/storage/registry.rs` (placeholder arm)
- Modify: `apps/manager/src/features/storage/config.rs` (placeholder arm)
- Modify: `apps/manager/src/features/storage_backends/health.rs` (placeholder arm)

- [ ] **Step 1**: Add `Smb` variant to both `BackendKind` enums + TS type union. Wire string is `"smb"`.
- [ ] **Step 2**: Add `BackendKind::Smb => Err(anyhow!("smb not yet implemented (Task 9)"))` placeholders in registry/config/health (same shape we used for iscsi_lvm).
- [ ] **Step 3**: `cargo build --release -p manager -p agent` must succeed.
- [ ] **Step 4**: Commit:
  ```
  git add -A
  git commit -m "feat(storage): add Smb backend kind variant + placeholder arms"
  ```

---

### Task 2: Migration 0039 — kind CHECK constraint

**Files:**
- Create: `apps/manager/migrations/0039_smb_backend_kind.sql`

- [ ] **Step 1**: Migration content:
  ```sql
  -- Allow 'smb' as a storage_backend.kind (parallel to nfs file-protocol backend).
  ALTER TABLE storage_backend
      DROP CONSTRAINT IF EXISTS storage_backend_kind_check;
  ALTER TABLE storage_backend
      ADD CONSTRAINT storage_backend_kind_check
      CHECK (kind IN ('local_file', 'iscsi', 'truenas_iscsi', 'spdk_lvol', 'nfs', 'iscsi_lvm', 'smb'));
  ```
- [ ] **Step 2**: Restart manager, watch startup log for migration apply.
- [ ] **Step 3**: Smoke test:
  ```
  PGPASSWORD=nexus psql -h 127.0.0.1 -p 5435 -U nexus -d nexus -c \
    "INSERT INTO storage_backend (name, kind, config_json, capabilities_json, is_default, source) \
     VALUES ('mig-smb-test', 'smb', '{}'::jsonb, '{}'::jsonb, false, 'ui') RETURNING id;"
  PGPASSWORD=nexus psql ... -c "DELETE FROM storage_backend WHERE name='mig-smb-test';"
  ```
- [ ] **Step 4**: Commit:
  ```
  git add apps/manager/migrations/0039_smb_backend_kind.sql
  git commit -m "feat(storage): migration 0039 — allow smb backend kind"
  ```

---

### Task 3: Agent — `mount.cifs` arg builders + helpers (pure-logic)

**Files:**
- Create: `apps/agent/src/features/storage/smb.rs`
- Test: inline

Pure-logic functions, no shell execution. TDD style — write failing test first.

- [ ] **Step 1**: Write the failing arg-builder tests:
  ```rust
  #[cfg(test)]
  mod arg_tests {
      use super::*;

      #[test]
      fn build_mount_args_authenticated() {
          let args = build_mount_args(
              "fileserver.local", "vms", None,
              Some("/etc/nqrust/storage-creds/abc.cred"),
              Some("vm-admin"), Some("CORP"), Some("3.0"), None,
              "/var/lib/nqrust/smb/abc",
          );
          let s = args.join(" ");
          assert!(s.contains("//fileserver.local/vms"));
          assert!(s.contains("/var/lib/nqrust/smb/abc"));
          assert!(s.contains("-t cifs"));
          assert!(s.contains("username=vm-admin"));
          assert!(s.contains("credentials=/etc/nqrust/storage-creds/abc.cred"));
          assert!(s.contains("domain=CORP"));
          assert!(s.contains("vers=3.0"));
      }

      #[test]
      fn build_mount_args_anonymous() {
          let args = build_mount_args(
              "srv", "public", None, None, None, None, None, None,
              "/var/lib/nqrust/smb/x",
          );
          let s = args.join(" ");
          assert!(s.contains("guest"));
          assert!(!s.contains("credentials="));
      }

      #[test]
      fn build_mount_args_with_subdir() {
          let args = build_mount_args(
              "srv", "share", Some("tenant-a"), None, None, None, None, None,
              "/var/lib/nqrust/smb/x",
          );
          let s = args.join(" ");
          assert!(s.contains("//srv/share/tenant-a"));
      }

      #[test]
      fn build_mount_args_ipv6_wraps_in_brackets() {
          let args = build_mount_args(
              "fe80::1", "share", None, None, None, None, None, None,
              "/var/lib/nqrust/smb/x",
          );
          assert!(args.iter().any(|a| a.contains("//[fe80::1]/share")));
      }

      #[test]
      fn build_mount_args_appends_extra_options() {
          let args = build_mount_args(
              "srv", "s", None, None, None, None, None,
              Some("uid=33,gid=33,file_mode=0660"),
              "/m",
          );
          let s = args.join(" ");
          assert!(s.contains("uid=33,gid=33,file_mode=0660"));
      }

      #[test]
      fn mount_point_for_uses_safe_share_chars() {
          // Same shape as NFS: server-safe + slugified share, deterministic
          let mp = mount_point_for("/var/lib/nqrust/smb", "192.168.1.5", "vm/data");
          assert_eq!(mp.to_string_lossy(), "/var/lib/nqrust/smb/192.168.1.5:vm_data");
      }
  }
  ```
- [ ] **Step 2**: Run — expect compile errors (functions undefined).
- [ ] **Step 3**: Implement:
  ```rust
  pub fn build_mount_args(
      server: &str,
      share: &str,
      subdir: Option<&str>,
      cred_file: Option<&str>,
      username: Option<&str>,
      domain: Option<&str>,
      smb_version: Option<&str>,
      extra_options: Option<&str>,
      mount_point: &str,
  ) -> Vec<String> {
      let server_h = if server.contains(':') { format!("[{server}]") } else { server.to_string() };
      let mut source = format!("//{server_h}/{share}");
      if let Some(s) = subdir { source.push('/'); source.push_str(s.trim_start_matches('/')); }

      let mut args: Vec<String> = vec![
          "-t".into(), "cifs".into(),
          source, mount_point.into(),
          "-o".into(), "soft".into(),
      ];
      let mut opts: Vec<String> = Vec::new();

      if let Some(u) = username {
          if let Some(cf) = cred_file {
              opts.push(format!("username={u}"));
              opts.push(format!("credentials={cf}"));
          } else {
              opts.push("guest".into());
              opts.push(format!("username={}", u.trim()));
          }
      } else {
          opts.push("guest".into());
          opts.push("username=guest".into());
      }
      if let Some(d) = domain { opts.push(format!("domain={d}")); }
      if let Some(v) = smb_version { opts.push(format!("vers={v}")); }
      if let Some(extra) = extra_options { opts.push(extra.to_string()); }

      args.push("-o".into());
      args.push(opts.join(","));
      args
  }

  pub fn mount_point_for(base: &str, server: &str, share: &str) -> std::path::PathBuf {
      let share_safe = share.trim_start_matches('/').replace('/', "_");
      let server_safe = server.replace([':', '/'], "_");
      std::path::PathBuf::from(base).join(format!("{server_safe}:{share_safe}"))
  }
  ```
- [ ] **Step 4**: Run `cargo test -p agent arg_tests` — all 6 pass.
- [ ] **Step 5**: Commit `feat(agent): smb mount.cifs arg builders + mount-point helper`.

---

### Task 4: Agent — credential file management

**Files:**
- Modify: `apps/agent/src/features/storage/smb.rs`

Cred file lives at `/etc/nqrust/storage-creds/<backend-id>.cred`, mode 0600, owned by root (agent user). Content matches `mount.cifs` credentials file format:
```
username=vm-admin
password=secret
domain=CORP
```

- [ ] **Step 1**: Test (inline):
  ```rust
  #[tokio::test]
  async fn cred_file_round_trip() {
      let tmp = tempfile::tempdir().unwrap();
      let path = tmp.path().join("test.cred");
      write_cred_file(&path, "user", "pass", Some("DOM")).await.unwrap();
      let perms = tokio::fs::metadata(&path).await.unwrap().permissions();
      assert_eq!(perms.mode() & 0o777, 0o600);
      let content = tokio::fs::read_to_string(&path).await.unwrap();
      assert!(content.contains("username=user"));
      assert!(content.contains("password=pass"));
      assert!(content.contains("domain=DOM"));
  }
  ```
- [ ] **Step 2**: Implement:
  ```rust
  pub async fn write_cred_file(
      path: &std::path::Path,
      username: &str,
      password: &str,
      domain: Option<&str>,
  ) -> std::io::Result<()> {
      use std::os::unix::fs::OpenOptionsExt;
      tokio::fs::create_dir_all(path.parent().unwrap_or_else(|| std::path::Path::new("/"))).await?;
      let mut f = tokio::fs::OpenOptions::new()
          .create(true).truncate(true).write(true)
          .mode(0o600)
          .open(path).await?;
      use tokio::io::AsyncWriteExt;
      f.write_all(format!("username={username}\npassword={password}\n").as_bytes()).await?;
      if let Some(d) = domain {
          f.write_all(format!("domain={d}\n").as_bytes()).await?;
      }
      Ok(())
  }

  pub async fn delete_cred_file(path: &std::path::Path) {
      let _ = tokio::fs::remove_file(path).await;
  }

  pub fn cred_file_path(backend_id: &uuid::Uuid) -> std::path::PathBuf {
      std::path::PathBuf::from("/etc/nqrust/storage-creds").join(format!("{backend_id}.cred"))
  }
  ```
- [ ] **Step 3**: Test passes.
- [ ] **Step 4**: Commit `feat(agent): smb credential-file helpers (chmod 0600)`.

---

### Task 5: Agent — mount/umount via `mount.cifs`

**Files:**
- Modify: `apps/agent/src/features/storage/smb.rs`

- [ ] **Step 1**: Async functions:
  ```rust
  pub async fn ensure_mounted(
      backend_id: uuid::Uuid,
      mount_base: &std::path::Path,
      server: &str,
      share: &str,
      subdir: Option<&str>,
      username: Option<&str>,
      domain: Option<&str>,
      smb_version: Option<&str>,
      extra_options: Option<&str>,
  ) -> Result<std::path::PathBuf, StorageError> {
      let mp = mount_point_for(mount_base.to_str().unwrap_or("/var/lib/nqrust/smb"), server, share);
      tokio::fs::create_dir_all(&mp).await?;
      // findmnt --mountpoint: exact match; if already mounted with the same source, return ok.
      let want_source = if subdir.is_some() {
          format!("//{}/{}/{}", server, share, subdir.unwrap_or(""))
      } else {
          format!("//{}/{}", server, share)
      };
      let probe = tokio::process::Command::new("findmnt")
          .args(["--mountpoint", mp.to_str().unwrap(), "-n", "-o", "SOURCE"])
          .output().await;
      if let Ok(out) = probe {
          if out.status.success() {
              let line = String::from_utf8_lossy(&out.stdout).trim().to_string();
              if !line.is_empty() && line == want_source {
                  return Ok(mp);
              }
              if !line.is_empty() {
                  return Err(StorageError::backend(std::io::Error::other(format!(
                      "{} is mounted but as '{}', not '{}'", mp.display(), line, want_source))));
              }
          }
      }

      let cred_file = if username.is_some() {
          let p = cred_file_path(&backend_id);
          if !tokio::fs::try_exists(&p).await.unwrap_or(false) {
              return Err(StorageError::backend(std::io::Error::other(format!(
                  "credentials file missing for backend {}: {}", backend_id, p.display()))));
          }
          Some(p.to_string_lossy().into_owned())
      } else { None };

      let args = build_mount_args(server, share, subdir, cred_file.as_deref(), username, domain, smb_version, extra_options, mp.to_str().unwrap());
      let status = tokio::process::Command::new("mount")
          .args(&args).status().await
          .map_err(|e| StorageError::backend(std::io::Error::other(format!("mount.cifs spawn: {e}"))))?;
      if !status.success() {
          return Err(StorageError::backend(std::io::Error::other(format!(
              "mount.cifs failed: exit {:?}; check credentials, smbversion, options", status.code()))));
      }
      Ok(mp)
  }

  pub async fn unmount(mp: &std::path::Path) -> Result<(), StorageError> {
      let _ = tokio::process::Command::new("umount").arg(mp).status().await;
      Ok(())
  }
  ```
- [ ] **Step 2**: No unit test for the live spawn (gated by env var if you want, `#[ignore]`).
- [ ] **Step 3**: Commit.

---

### Task 6: Agent — file lifecycle (create/delete/clone-from-path/snapshot)

**Files:**
- Modify: `apps/agent/src/features/storage/smb.rs`

Identical shape to NFS — operate on `<mount-point>/<file>`. Validates `file` is a plain filename (no `/`, no leading `.`).

- [ ] **Step 1**: Test the locator validation:
  ```rust
  #[test]
  fn locator_rejects_slash() {
      assert!(SmbLocator::validate_file("foo.raw").is_ok());
      assert!(SmbLocator::validate_file("foo/bar").is_err());
      assert!(SmbLocator::validate_file(".hidden").is_err());
      assert!(SmbLocator::validate_file("").is_err());
  }
  ```
- [ ] **Step 2**: Implement `create_file`, `delete_file`, `clone_from_path`, `snapshot`, `clone_from_snapshot` — direct copy/adapt of `apps/agent/src/features/storage/nfs.rs` equivalents.
- [ ] **Step 3**: Commit `feat(agent): smb file lifecycle (create/delete/clone/snapshot)`.

---

### Task 7: Agent — HTTP routes + host-backend registration

**Files:**
- Modify: `apps/agent/src/features/storage/smb.rs` (add `pub fn router`)
- Modify: `apps/agent/src/features/storage/routes.rs` (nest `/smb`)
- Modify: `apps/agent/src/main.rs` (register `SmbHostBackend` if `mount.cifs` exists)

Routes (all POST):
| Path | Body | Returns |
|---|---|---|
| `/set_credentials` | `{backend_id, username, password, domain?}` | 204 |
| `/clear_credentials` | `{backend_id}` | 204 |
| `/mount` | `{backend_id, server, share, subdir?, username?, domain?, smb_version?, options?}` | `{mount_point}` |
| `/umount` | `{server, share, subdir?}` | 204 |
| `/create_file` | `{server, share, subdir?, file, size_bytes}` | 204 |
| `/delete_file` | `{server, share, subdir?, file}` | 204 |
| `/clone_from_path` | `{source_path, server, share, subdir?, file}` | 204 |
| `/snapshot` | `{server, share, subdir?, source_file, snap_file}` | 204 |
| `/clone_from_snapshot` | `{server, share, subdir?, snap_file, file}` | `{size_bytes}` |

`SmbHostBackend` implements `HostBackend` — `attach()` returns `AttachedPath::File(...)` pointing at `<mount-point>/<file>`.

- [ ] **Step 1**: Wire routes.
- [ ] **Step 2**: Build green.
- [ ] **Step 3**: Commit `feat(agent): HTTP routes for smb backend + host-backend registration`.

---

### Task 8: Manager — `SmbConfig` + `SmbControlPlaneBackend`

**Files:**
- Create: `apps/manager/src/features/storage/backends/smb.rs`
- Modify: `apps/manager/src/features/storage/backends/mod.rs`

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct SmbConfig {
    pub server: String,
    pub share: String,
    #[serde(default)] pub subdir: Option<String>,
    #[serde(default)] pub username: Option<String>,
    #[serde(default)] pub domain: Option<String>,
    #[serde(default)] pub smb_version: Option<String>,
    #[serde(default)] pub options: Option<String>,
    #[serde(default)] pub mount_base: Option<PathBuf>,
    #[serde(default)] pub assume_mounted: bool,
    pub agent_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmbLocator {
    pub server: String,
    pub share: String,
    pub subdir: Option<String>,
    pub file: String,
}

pub struct SmbControlPlaneBackend {
    pub id: BackendInstanceId,
    pub config: SmbConfig,
}
```

Trait impl mirrors NFS:
- `provision`/`destroy` → agent's `/create_file` and `/delete_file`
- `clone_from_image` → agent's `/clone_from_path`
- `snapshot`/`clone_from_snapshot`/`delete_snapshot` → agent's snapshot routes
- `probe` → agent's `/mount` (best-effort idempotent)
- `host_path_for` → `<mount_base>/<server>:<share>/<file>` (via locator parse)
- `capabilities` → `{clone_from_image: true, native_snapshots: false (cp-based, not CoW), concurrent_attach: false, live_migration: false}`

- [ ] Tests: config parse + locator round-trip + capabilities + host_path_for resolution.
- [ ] Commit `feat(manager): smb control-plane backend (delegates to agent)`.

---

### Task 9: Manager — wire into registry/config/health (replace placeholders)

**Files:**
- Modify: `apps/manager/src/features/storage/registry.rs`
- Modify: `apps/manager/src/features/storage/config.rs`
- Modify: `apps/manager/src/features/storage_backends/health.rs`

- registry: arm decodes `SmbConfig`, fills `agent_url` from `default_agent_url`, constructs `SmbControlPlaneBackend`.
- config validate: require `server` + `share`. If `username` present, password must also be supplied (separately, via the create handler — config row only holds non-secret fields). If both empty → guest mode.
- health: probe TCP 445 reachable + `mount.cifs` returns ok. Compute used/total via `df` of the mount point.

- [ ] Strip all `"smb not yet implemented"` placeholders.
- [ ] Commit `feat(manager): wire smb into registry/config/health`.

---

### Task 10: Manager — `create`/`update` handlers send credentials to agent

**Files:**
- Modify: `apps/manager/src/features/storage_backends/routes.rs`

The `create` handler currently does: validate → upsert → probe (NFS-only). For SMB:
1. Validate (config_json has non-secret fields only)
2. **If kind == "smb" AND request body has `password`** → POST agent `/v1/storage/smb/set_credentials` with `{backend_id, username, password, domain}` BEFORE the probe. (The UI passes password in a separate top-level field from the regular `config` blob, so it never lands in `config_json`.)
3. Upsert backend row (no password in DB)
4. Probe (mounts using the cred file)
5. On any failure → rollback (soft-delete row + clear cred file)

Similarly for `update` — if password is present, refresh cred file on agent.

`delete` handler → POST `/v1/storage/smb/clear_credentials` after soft-delete.

- [ ] Tests: probe_smb_backend helper mirrors probe_nfs_backend.
- [ ] Commit `feat(manager): smb credential delivery + probe in create/update/delete`.

---

### Task 11: UI — `smb` form schema with new `password`/`select` field types

**Files:**
- Modify: `apps/ui/components/storage/backend-create-dialog.tsx`
- Modify: `apps/ui/components/storage/backend-table.tsx` (KIND_LABEL)
- Modify: `apps/ui/lib/types/index.ts`

Add to `Field`:
```ts
type FieldType = "string" | "boolean" | "password" | "select";
interface Field {
  // existing...
  type?: FieldType;
  options?: string[]; // for select
}
```

Render password as `<Input type="password" autoComplete="new-password">`. Render select as the existing shadcn Select with `options` enum.

SMB field schema:
- `server` (required), `share` (required)
- `username` (required, but blank = guest)
- `password` (required, type=password, blank = guest)
- `domain`, `smb_version` (advanced, select with `default/2.0/2.1/3/3.0/3.11`)
- `subdir`, `options` (advanced, strings)
- `mount_base` (advanced)

Submit payload — separate the password from the rest:
```ts
const submitBody = {
  name,
  kind: "smb",
  is_default,
  config: { server, share, subdir, username, domain, smb_version, options, mount_base },
  // Password is a sibling field (manager extracts + sends to agent, never persists):
  ...(password ? { password } : {}),
};
```

Backend type extension for `CreateStorageBackendReq`:
```rust
pub struct CreateStorageBackendReq {
    pub name: String,
    pub kind: BackendKind,
    #[serde(default)] pub is_default: bool,
    #[serde(default)] pub config: JsonValue,
    /// SMB-only: non-empty triggers `set_credentials` on the agent. Never stored.
    #[serde(default)] pub password: Option<String>,
}
```

- [ ] `pnpm build` must succeed.
- [ ] Visual check (Storage → Add Backend → SMB shows the form correctly).
- [ ] Commit `feat(ui): smb form fields + new password/select field types`.

---

### Task 12: Edit-in-place — password rotation

**Files:**
- Modify: `apps/ui/components/storage/backend-edit-dialog.tsx`

Existing edit dialog round-trips `config_json` via `GET /:id/config`. For SMB, add an optional "Change password" subsection — leave blank to keep current creds, fill to rotate. On Save, manager sends the new password to agent's `set_credentials`.

- [ ] Commit `feat(ui): smb password rotation in edit dialog`.

---

### Task 13: Host packages

**Files:**
- Modify: `scripts/install/lib/deps.sh` (add `cifs-utils` to apt + dnf paths)
- Modify: `apps/installer/src/installer/deps.rs` (same)
- Modify: `scripts/airgap/bundle-debs-ubuntu.sh` (same)

- [ ] Commit `feat(install): bundle cifs-utils for smb backend`.

---

### Task 14: Docs

**Files:**
- Create: `docs/runbooks/smb-troubleshooting.md`
- Modify: `CHANGELOG.md` (Unreleased → Added section)
- Modify: `README.md` (Storage Backends table: add SMB row alongside NFS)

Runbook covers:
- "mount.cifs failed: exit 13" — auth failure (wrong user/pass/domain)
- "mount.cifs failed: exit 32" — share doesn't exist on server
- "mount.cifs: bad option" — version mismatch, try other `smbversion`
- "Permission denied writing to share" — uid/gid mismatch, set in `options`
- "Edit-in-place: password unchanged after save" — blank password field = keep existing, must explicitly fill to rotate
- "Anonymous shares not mounting" — server may require `username=guest` instead of `-o guest`

- [ ] Commit `docs(storage): smb troubleshooting runbook + CHANGELOG entry + README row`.

---

### Task 15: E2E live verification in the iscsi-alpha VM

The VM at 10.42.0.68 (or recreate if stale) gets a local Samba target:

```bash
ssh root@<vm-ip>
DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends samba samba-common-bin
mkdir -p /var/lib/test-smb/share
chmod 777 /var/lib/test-smb/share
cat > /etc/samba/smb.conf <<EOF
[global]
   workgroup = WORKGROUP
   server min protocol = SMB2
   server max protocol = SMB3
   server signing = auto
[vms]
   path = /var/lib/test-smb/share
   read only = no
   guest ok = yes
   force user = root
EOF
systemctl restart smbd
useradd vm-admin && (echo "smb-pass"; echo "smb-pass") | smbpasswd -a -s vm-admin
```

Test runner (`infra/test/smb-runner.sh`):
- T1 Create backend (authenticated): name, kind=smb, server=127.0.0.1, share=vms, username=vm-admin, password=smb-pass → 201
- T2 Validation: missing share/server → 400; password without username → 400
- T3 Probe: backend health.reachable=true after a few seconds
- T4 Anonymous backend: same but username=password="" → mounts with `-o guest`
- T5 VM lifecycle: create VM on smb backend, verify rootfs file exists at `/var/lib/nqrust/smb/<key>/...`, stop/start/delete cycle clean
- T6 Edit-in-place rotation: PUT new password → agent's cred file updated → manager re-probes
- T7 Live registry: delete backend, agent clears cred file, mount torn down
- T8 Wrong-credential rejection: create with bad password → 422 with the mount.cifs error in the toast

- [ ] Run, expect all green.
- [ ] If failures, capture in followups + fix critical ones.
- [ ] Commit test runner + KubeVirt-side setup snippet.

---

### Task 16: Bump to v0.4.0 + ship

**Files:**
- Modify: `apps/*/Cargo.toml`, `apps/ui/package.json` (0.3.0 → 0.4.0)
- Modify: `CHANGELOG.md` ([Unreleased] → [0.4.0])

- [ ] `cargo build --release --workspace` clean.
- [ ] Push, tag `v0.4.0`, watch release pipeline.
- [ ] Mark Latest on GitHub.

---

## Verification (end-to-end)

After Task 16:
1. Download `nqrust-manager-x86_64-linux-musl` from the v0.4.0 release.
2. Run the SMB integration suite against it inside the test VM.
3. All 8 test groups green → call it done.

If anything regresses on the iscsi_lvm side, re-run that suite too — should be 23/23 unchanged.

---

## Out of Scope (explicit punts)

1. **Kerberos auth UI** — keytab management is a separate feature; ops can hand-edit `options` to use `sec=krb5` for now.
2. **Browse share contents** — listing files inside the SMB share from the UI; nice future feature.
3. **DFS namespaces** — Microsoft DFS referrals work via mount.cifs's default behaviour but we don't surface them as a first-class kind.
4. **Refactor file-share abstraction** — NFS and SMB are now sister backends with ~70% identical code shapes. After SMB ships, an extraction pass can dedupe them. Deferred to keep this sprint scope tight.
