# NQRust-MicroVM: Chunked Encrypted Backup Pipeline

**Status:** Design
**Date:** 2026-04-29
**Owner:** kleopasevan
**Scope:** Backup + restore for any volume on any backend, to any S3-compatible target, with content-addressed deduplication, per-target convergent encryption, and daily mark-and-sweep GC. First half of "Path B" from the storage HCI roadmap (the SPDK + Raft block backend is the second, separate spec).

## Context

The storage foundation (PR #14, merged 2026-04-28) shipped a `ControlPlaneBackend::snapshot()` method on the trait, with native instant snapshots on TrueNAS-iSCSI and a slow-but-correct `fs::copy` fallback on LocalFile. iSCSI generic and (future) SPDK lvol both have native snapshots too.

What we don't yet have:

- A way to *export* those snapshots off the host. They live on whatever the backend stores them on. If the host dies, the snapshots die with it.
- A backup target abstraction. Operators can't say "back up this volume to S3."
- Deduplication. Two VMs cloned from the same image waste 100% of one VM's storage on every snapshot.
- A retention policy. Snapshots accumulate forever.

This spec adds all four.

## Intent

Build the chunked-deduplicated-encrypted backup tier the original HCI spec deliberately deferred (`docs/superpowers/specs/2026-04-28-storage-hci-design.md` line 274). Bring-your-own S3 target. Agent-side chunker (no manager bandwidth bottleneck). Per-target convergent encryption (S3-side compromise reveals nothing; cross-volume dedup intact). Per-volume cron-style schedules with retention. Daily mark-and-sweep GC.

This is **half of Path B**. The other half (SPDK + Raft distributed block backend) is a separate spec → plan → PR cycle. The two are independently shippable and will be parallel-tracked.

## Why this matters

- **Today**: every snapshot is host-local. Lose a host, lose the snapshots. There is no off-host backup story.
- **After this PR**: every backend (LocalFile, Iscsi, TrueNasIscsi, future SPDK) can back up to any S3 target. Backups are content-addressed and deduplicated globally per target. Encrypted at rest with a key the operator owns.
- **Strategic**: enterprise/government customers require off-host immutable backups before they'll trust the platform. This makes the "but where do my backups live" question have a credible answer that doesn't require a second proprietary system.
- **Forward-compatible with Path B SPDK**: when SPDK lvol arrives with native snapshots, it implements `snapshot()` + `read_snapshot()` and gets the entire backup pipeline for free.

## Scope

### In this PR

1. **New crate `nexus-backup`** — pure-Rust transforms: FastCDC chunking, BLAKE3 hashing, XChaCha20-Poly1305 encryption, manifest serialization. No I/O. Used by both manager and agent.
2. **One trait method addition** to `nexus_storage::HostBackend`: `read_snapshot(&self, snap: &VolumeSnapshotHandle) -> Box<dyn AsyncRead + Send + Unpin>`. Implemented for LocalFile, Iscsi, TrueNasIscsi.
3. **Schema migration `0036_backup.sql`**: tables `backup_target`, `backup`, `backup_gc_run`. Volume gains `backup_cron`, `backup_retain_count`, `backup_target_id`.
4. **Manager feature module `apps/manager/src/features/backup_targets/`** — CRUD for target rows; manages encrypted secret_access_key + per-target encryption key under the existing envelope-key scheme used by SSO.
5. **Manager feature module `apps/manager/src/features/backups/`** — backup row lifecycle, scheduler tokio task (cron-driven, per-volume), GC tokio task (daily mark-and-sweep per target), reconciler tokio task (re-kicks stale `running` rows after a configurable timeout), REST endpoints for create / list / get / restore / delete.
6. **New agent HTTP routes** (under existing `/v1/storage/...`): `POST /v1/storage/backup`, `POST /v1/storage/restore`. Carry the decrypted target key in-memory only; never persist or log it.
7. **Manager-side agent RPC helpers** in `apps/manager/src/features/storage/agent_rpc.rs`: `agent_backup`, `agent_restore`.
8. **UI** in `apps/ui`: backup-targets admin page (CRUD); per-volume "Backups" tab listing this volume's backups + manual "Backup now" + restore-to-new-volume button + per-volume schedule editor.
9. **Index-rebuild tool** — manager subcommand `nqrust backup index-rebuild --target <id>` that reconstructs DB rows from S3 manifests for DR scenarios where the DB is lost.
10. **Tests** per the testing strategy section below.

### Explicitly out of scope

Design the abstraction so these slot in cleanly later, but do not implement:

- Embedded SeaweedFS lifecycle (deploy + cluster + master HA from the manager). Operator brings their own S3 endpoint in v1.
- Restore-in-place / rollback to existing volume. The trait already deliberately omits `rollback_to_snapshot`; the same logic extends here. New-volume restore only.
- Cluster-wide backup policies (label/tag selectors). Per-volume schedules only.
- Cross-region replication of the backup target itself.
- Live (non-snapshot) backup of running VMs with application quiescing.
- Selector-based exclude lists (skip /tmp, /var/cache, etc.). Whole-volume backups only.
- Resumable chunking with mid-stream seek. Implicit resume via content-addressing only (Q7).
- Bandwidth throttling per-target.
- Application-aware quiescing (fsfreeze, MySQL flush, pg_basebackup). Operator's responsibility.
- Backup verification beyond "PUT succeeded" (e.g., sample-restore-and-checksum). Future enhancement.

## Architectural intent (constraints, not implementation)

### Where work happens

- **Chunking and PUTs run on the agent** (whichever host is currently authoritative for the volume's snapshot). Manager bytes never traverse the manager↔agent link more than once. Bandwidth scales linearly with the number of agent hosts.
- **Manager orchestrates and tracks state**: which backups are running, which target they go to, what schedule they're on, when GC last ran. The manager DB is authoritative for *state*; S3 is authoritative for *data*.
- **Encryption keys travel one-way only**: manager decrypts → pushes to agent over RPC → agent uses in-memory → agent process exit clears them. Never persisted on the agent.

### Two encryption layers

- **Envelope key** (already exists for SSO): a 32-byte AES-GCM key on the manager. Wraps two things per `backup_target`:
  - the S3 secret access key (so a DB leak doesn't compromise the bucket),
  - the per-target chunk key (XChaCha20-Poly1305, 32 bytes).
- **Per-target chunk key**: encrypts every chunk and the manifest. Never leaves manager memory except during a backup/restore RPC to the agent.

### Convergent encryption preserves dedup

- Per chunk: `nonce = first 24 bytes of BLAKE3(plaintext)`. Same plaintext → same nonce → same ciphertext (under the same target key) → same `BLAKE3(ciphertext)` → same S3 object key → dedup hit.
- Manifest: random nonce. The manifest is per-backup unique by construction; deterministic nonce buys nothing for it.
- Tradeoff: convergent encryption is vulnerable to "confirmation of file" — an attacker with a known plaintext + the target key can confirm the cluster also has it. For VM-disk backups this is the right tradeoff; the real threat is bucket compromise, which convergent + per-target-key defends against.

### Trust boundaries

- **Manager**: holds envelope key (in memory at startup, decrypted from `MANAGER_ENVELOPE_KEY` env or KMS), DB credentials, all secrets. Single trust authority.
- **Agent**: trusted by the manager to receive an in-memory key for one operation. After the operation, the key is dropped. An agent compromised mid-operation can read backups for that one target, not the whole fleet.
- **S3 endpoint**: untrusted. Sees only ciphertext + content-addressed object keys. Can't distinguish a backup from random data.

### `read_snapshot` is the only new trait method

The whole pipeline composes from existing trait methods plus this one addition:

- Backup: `cp.snapshot(volume)` → `host.read_snapshot(snap)` → chunker pipeline.
- Restore: `cp.provision(target_size)` → `host.attach(volume)` → reverse pipeline writes via the attached path.

Adding `read_snapshot` to existing `HostBackend` impls:

- **LocalFile**: `tokio::fs::File::open(snap.locator)` — the snapshot is just a file.
- **Iscsi/TrueNasIscsi**: same iSCSI logic as `attach`, but for the snapshot LUN. TrueNAS REST exposes ZFS snapshots as separate iSCSI extents; the locator JSON gets a `snapshot_lun` field. For generic iSCSI, operator must pre-create a LUN exposing the snapshot (or the impl returns `NotSupported` and falls back to `clone_from_snapshot` + `read` of the new volume).
- **Future SPDK**: lvol snapshots are first-class; trivial to expose as a read-only NVMe-oF target.

### Manifest is canonical-in-S3, mirror-in-DB

- DB has the queryable metadata (size, chunk count, status, timestamps, target_id, source_volume_id). Fast UI listing.
- S3 has the full chunk list under `manifests/<backup-uuid>.bin`. DR-safe — losing the DB doesn't lose the backups.
- `nqrust backup index-rebuild` reconstructs DB from S3.

### Garbage collection: mark-and-sweep, daily, per-target

- For each target: read all manifests → union all chunk hashes → list all `chunks/` objects → delete any whose hash isn't in the union. Standard pattern (restic, BorgBackup, Kopia all do this).
- Safety: chunks <24h old are excluded from sweep candidacy (protects in-flight backups whose manifest hasn't been written yet).
- Reconciler: runs every 5 min, marks `running` backups older than 24h as `failed` so their orphans become reachable to GC.

### Implicit resume via content-addressing

- Chunks are keyed by `BLAKE3(ciphertext)`. The HEAD-before-PUT pattern means a retry naturally skips already-uploaded chunks.
- Crashed manager mid-backup → reconciler picks up the `running` row → re-dispatches → most chunks 200-on-HEAD → finishes quickly.
- We do NOT implement resumable chunking with mid-stream seek (Q7). Re-chunking the source on retry is bounded waste (~5 min for 100GB at 350 MB/s BLAKE3 single-threaded); the win from skipping seek-checkpoint complexity is much larger than the loss from re-chunking.

### Per-volume schedules, not cluster-wide policies

- `volume.backup_cron`, `volume.backup_retain_count`, `volume.backup_target_id` are per-volume columns.
- Manager runs one tokio task that wakes on the next-fire time across all schedules; on fire, it dispatches a backup. Standard cron parsing via the `cron` crate.
- Retention enforcement is part of the same task: after a successful backup, count the volume's `completed` backups; if > retain_count, mark the oldest `pruning`, delete its manifest object, delete its DB row. Chunks become reachable to next GC cycle.
- Cluster-wide label-selector policies (Q3 option C) are deferred. Adding them later is a thin layer over per-volume schedules — derive `backup_cron` for matching volumes.

### Backwards compatibility

- The existing `snapshot()` / `clone_from_snapshot()` / `delete_snapshot()` trait methods are unchanged. Existing TrueNAS-snapshot-stub returns continue to be `NotSupported` (the foundation spec's call); they're orthogonal to this work.
- All existing volumes continue to function. The new `volume.backup_*` columns are nullable with no default behavior; volumes without a configured schedule are simply never auto-backed-up (and can be backed up manually if and when the operator configures a target).
- Any backend that doesn't yet implement `read_snapshot` returns `StorageError::NotSupported("read_snapshot")` and the backup attempt fails with a clear "this backend doesn't support backup yet" message. The chunk pipeline doesn't crash — it surfaces a typed error.

## Trait surface

```rust
// crates/nexus-storage/src/host.rs — addition
#[async_trait]
pub trait HostBackend: Send + Sync {
    // ... existing methods unchanged ...

    /// Open a snapshot for reading. Returns a stream of bytes representing
    /// the volume contents at snapshot time. Caller (the agent's backup
    /// pipeline) reads the stream linearly, chunks it, encrypts each chunk,
    /// and PUTs to S3.
    ///
    /// Implementations:
    /// - LocalFile: open the snapshot file from disk.
    /// - Iscsi/TrueNasIscsi: attach the snapshot LUN (read-only) and return
    ///   a File handle over /dev/disk/by-path/...
    /// - Future SPDK: open the lvol snapshot via NVMe-oF.
    async fn read_snapshot(
        &self,
        snap: &VolumeSnapshotHandle,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>, StorageError>;
}
```

No additions on `ControlPlaneBackend`. The existing `provision`, `snapshot`, `delete_snapshot` are sufficient.

## Schema (migration `0036_backup.sql`)

```sql
-- 0036_backup.sql

CREATE TABLE IF NOT EXISTS backup_target (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL UNIQUE,
  endpoint TEXT NOT NULL,
  region TEXT,
  bucket TEXT NOT NULL,
  prefix TEXT NOT NULL DEFAULT '',
  access_key_id TEXT NOT NULL,
  encrypted_secret_access_key BYTEA NOT NULL,
  encrypted_target_key BYTEA NOT NULL,
  gc_hour SMALLINT NOT NULL DEFAULT 3 CHECK (gc_hour BETWEEN 0 AND 23),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  deleted_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS backup (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  source_volume_id UUID REFERENCES volume(id) ON DELETE SET NULL,
  source_snapshot_id UUID,
  target_id UUID NOT NULL REFERENCES backup_target(id),
  manifest_object_key TEXT,
  size_bytes BIGINT NOT NULL DEFAULT 0,
  unique_bytes BIGINT NOT NULL DEFAULT 0,
  chunk_count BIGINT NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'running'
    CHECK (status IN ('running', 'completed', 'failed', 'pruning')),
  error_message TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  completed_at TIMESTAMPTZ,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_backup_volume ON backup(source_volume_id);
CREATE INDEX idx_backup_target ON backup(target_id);
CREATE INDEX idx_backup_status_updated ON backup(status, updated_at)
  WHERE status = 'running';

ALTER TABLE volume ADD COLUMN IF NOT EXISTS backup_cron TEXT;
ALTER TABLE volume ADD COLUMN IF NOT EXISTS backup_retain_count INT;
ALTER TABLE volume ADD COLUMN IF NOT EXISTS backup_target_id UUID
  REFERENCES backup_target(id);

CREATE TABLE IF NOT EXISTS backup_gc_run (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  target_id UUID NOT NULL REFERENCES backup_target(id) ON DELETE CASCADE,
  started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  completed_at TIMESTAMPTZ,
  bytes_freed BIGINT NOT NULL DEFAULT 0,
  chunks_deleted BIGINT NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'running'
    CHECK (status IN ('running', 'completed', 'failed')),
  error_message TEXT
);
CREATE INDEX idx_backup_gc_run_target ON backup_gc_run(target_id, started_at DESC);

COMMENT ON COLUMN backup_target.encrypted_secret_access_key IS
  'AES-GCM(envelope_key) over the S3 secret access key.';
COMMENT ON COLUMN backup_target.encrypted_target_key IS
  'AES-GCM(envelope_key) over the per-target XChaCha20-Poly1305 key used for chunk + manifest encryption.';
COMMENT ON COLUMN backup.unique_bytes IS
  'Post-dedup ciphertext bytes that this backup actually wrote (chunks not skipped by HEAD). Useful for billing.';
```

## API surface

```
POST   /v1/backup_targets                     # create target (operator provides S3 creds)
GET    /v1/backup_targets                     # list active targets
GET    /v1/backup_targets/{id}                # detail
PATCH  /v1/backup_targets/{id}                # update endpoint/creds
DELETE /v1/backup_targets/{id}                # soft-delete (rejected if backups reference it)

POST   /v1/backup_targets/{id}/gc             # trigger ad-hoc GC

POST   /v1/volumes/{id}/backup                # one-shot backup → returns backup id
PATCH  /v1/volumes/{id}/backup_schedule       # set cron + retention + target

GET    /v1/backups                            # list (paged; filter by volume_id, target_id, status)
GET    /v1/backups/{id}                       # detail
DELETE /v1/backups/{id}                       # mark for pruning; manifest deleted; chunks GC'd later
POST   /v1/backups/{id}/restore               # → returns new volume id

POST   /v1/backups/{id}/progress              # internal — agent → manager during long backup
                                              # (manager-issued bearer token specific to this backup)
```

## S3 layout

```
s3://<bucket>/<prefix>/
  manifests/<backup-uuid>.bin       # zstd-compressed bincode-serialized Manifest, encrypted with target key
  chunks/<hash[0..2]>/<hash>        # chunk content addressed by BLAKE3(ciphertext); ciphertext is XChaCha20-Poly1305(zstd(plaintext))
```

The `<hash[0..2]>` two-char prefix sharding is for S3 implementations that handle wide flat namespaces poorly (notably some on-prem). AWS, MinIO, and SeaweedFS handle flat namespaces fine but the prefix is harmless.

## Error handling

| Failure | Detection | Response |
|---|---|---|
| Agent crashes mid-backup | RPC connection-reset / timeout | `backup.status='failed'` + error message; reconciler re-queues if configured. |
| Network blip during chunk PUT | reqwest transient error | Per-chunk retry (5 attempts, exponential backoff). All-fail → abort backup. |
| S3 returns 403 | first PUT/HEAD response | Abort immediately, `failed` with "auth" message. No retry. |
| Source snapshot disappears mid-read | `read_snapshot` IO error | Abort, `failed`. Half-written chunks orphaned but content-addressed → next GC cleans. |
| Manager restarts mid-backup | DB has `running` row | Reconciler on startup re-dispatches. HEAD-before-PUT skips done chunks. |
| Target DELETE while backup in-flight | FK constraint | DELETE rejected: 409 "target has N backups; delete them first." |
| GC race with active backup | Mitigation: GC ignores chunks <24h | Worst case: re-PUT next time chunk needed. No corruption. |
| Manifest in S3 but DB row lost | DR scenario | `nqrust backup index-rebuild --target <id>` reconstructs DB rows. |
| GC fails mid-sweep | `backup_gc_run.status='failed'` | Next day's run retries. Storage temporarily oversized. |

Two error-handling principles:

1. Content-addressing makes idempotency free. Re-running anything is safe — worst case is wasted CPU/bandwidth.
2. Manager DB is authoritative for state, S3 is authoritative for data. The rebuild tool reconciles them.

## Open questions to resolve during implementation

These are tactical decisions; flag them in the PR description. None require revisiting the architecture:

- **FastCDC parameters** (min, avg, max chunk size). Default to (4KB, 64KB, 1MB) — what restic uses; well-studied for VM-disk-style workloads. Operator can override per-target via `chunker_params` JSON column if needed (not in v1 schema; add if requested).
- **zstd compression level**. Default 3. Higher levels (10+) get diminishing returns and significantly more CPU. Trade-off worth exposing per-target later.
- **S3 client choice**: `aws-sdk-s3` (heavy, exact compat with AWS) vs `rusoto` (deprecated) vs `s3` (lightweight). Recommend `aws-sdk-s3` because it works against any S3 implementation worth using and the binary-size hit is acceptable.
- **Concurrency limits**: how many chunks PUT in parallel per backup; how many backups run in parallel per agent; how many GC sweeps run in parallel. Start with conservative defaults (8 PUTs per backup, 2 backups per agent, 1 GC per cluster) configurable via env vars.
- **Progress reporting cadence**: every N MB? every N chunks? every N seconds? Default to every 64MB or 30s, whichever first.
- **STS / scoped creds**: detect if the target endpoint supports STS-style temporary credentials (AWS yes, MinIO yes via STS API, SeaweedFS partial). Use them when available; fall back to long-lived creds otherwise. Recommendation: implement long-lived first (works everywhere); add STS in a follow-up.
- **Per-target rate limit on GC** to keep it from saturating the S3 endpoint during business hours. Default: 100 ops/sec. Tunable.
- **Backup verification mode**: a "verify after backup" option that downloads N random chunks and asserts their BLAKE3. Useful for high-value targets. Off by default.

## File-level outline

New crate:
- `crates/nexus-backup/Cargo.toml` (deps: blake3, xchacha20poly1305 via `chacha20poly1305`, fastcdc, zstd, bincode, thiserror, async-trait)
- `crates/nexus-backup/src/lib.rs` — re-exports
- `crates/nexus-backup/src/chunker.rs` — FastCDC wrapper that yields `Chunk { plaintext_offset, plaintext_length, plaintext_bytes }` over an `AsyncRead`.
- `crates/nexus-backup/src/cipher.rs` — XChaCha20-Poly1305 encrypt/decrypt with convergent nonce derivation.
- `crates/nexus-backup/src/manifest.rs` — `Manifest`, `ChunkRef`, `bincode_serialize_compressed_encrypted`, inverse.
- `crates/nexus-backup/src/error.rs` — `BackupError`.

Manager additions:
- `apps/manager/src/features/backup_targets/{mod,repo,routes}.rs`
- `apps/manager/src/features/backups/{mod,repo,routes}.rs` — also houses the scheduler, GC, and reconciler tasks.
- `apps/manager/src/features/storage/agent_rpc.rs` — extend with `agent_backup`, `agent_restore`.
- `apps/manager/src/features/backups/index_rebuild.rs` — DR tool.
- `apps/manager/migrations/0036_backup.sql`.

Manager modifications:
- `apps/manager/Cargo.toml` — add `nexus-backup`, `aws-sdk-s3` (or chosen S3 client), `cron`.
- `apps/manager/src/main.rs` — start scheduler + GC + reconciler tokio tasks.

Agent additions:
- `apps/agent/src/features/storage/backup.rs` — pipeline implementation.
- `apps/agent/src/features/storage/routes.rs` — extend with `backup`, `restore` handlers.

Agent modifications:
- `apps/agent/Cargo.toml` — add `nexus-backup`, S3 client.

Trait modification:
- `crates/nexus-storage/src/host.rs` — add `read_snapshot` method.
- `apps/manager/src/features/storage/backends/{local_file,iscsi_generic,truenas_iscsi}.rs` — implement `read_snapshot` on each existing impl in the manager.
- `apps/agent/src/features/storage/{local_file,iscsi}.rs` — implement `read_snapshot` on each existing impl in the agent.

Shared types:
- `crates/nexus-types/src/lib.rs` — add `BackupTarget` (wire), `Backup` (wire), `BackupSchedule`.

UI:
- `apps/ui/lib/types/index.ts` — `BackupTarget`, `Backup`, `BackupSchedule`.
- `apps/ui/lib/queries.ts` — `useBackupTargets`, `useBackups(volumeId)`, `useBackup(backupId)`, mutations for create/restore/delete.
- `apps/ui/components/backup/backup-target-form.tsx`
- `apps/ui/components/backup/backup-list.tsx`
- `apps/ui/components/backup/backup-schedule-editor.tsx`
- `apps/ui/components/backup/restore-dialog.tsx`
- `apps/ui/app/(dashboard)/backup-targets/page.tsx`
- Volume detail page gains a "Backups" tab embedding the above components.

Configuration:
- `MANAGER_ENVELOPE_KEY` env var — already used by SSO, reused here.
- New TOML section `[backup]` (optional, single section, not array): `gc_concurrency`, `backup_concurrency_per_agent`, `chunk_concurrency_per_backup`, `progress_report_interval_seconds`. All have sane defaults; section is optional.

## Testing strategy

- **Pure transforms (unit, no I/O)**: `nexus-backup` crate gets exhaustive tests of FastCDC determinism, BLAKE3 round-trip, XChaCha20-Poly1305 round-trip, convergent encryption (same plaintext → same ciphertext under same key), manifest serialization round-trips. ~10 tests.
- **Trait round-trip**: write known bytes via `populate_streaming` → snapshot → `read_snapshot` → assert bytes-equal. Exercised on LocalFile (file I/O), Iscsi (block-device I/O when iscsi sim is up — `--ignored`).
- **Integration with mockito S3**: full backup → restore cycle on a 4MB pseudo-random source. Assert byte-identical restore. Assert manifest object exists in mock. Assert chunks are content-addressed.
- **Dedup**: run the same backup twice. Second run should be ~instant (only manifest PUT, all chunks HEAD-200).
- **Crash recovery**: kill agent mid-backup after some chunks PUT, before manifest written. Reconciler picks up `running` row, re-dispatches; second attempt completes; manifest is written; `unique_bytes` second-attempt is small (only re-PUTs missing chunks).
- **GC**: run a backup, delete its DB row, run GC, assert chunk count drops. Run two backups sharing 50% chunks, delete one, assert only the unique chunks vanish.
- **Encryption tamper**: tamper with one chunk's ciphertext in the mock S3, attempt restore, assert Poly1305 MAC failure surfaces as `RestoreError::CorruptChunk`.
- **Index rebuild**: TRUNCATE backup table, run `index-rebuild`, assert row count and metadata match pre-truncate.
- **Schedule + retention**: set retain_count=2, run 5 backups, assert oldest 3 are pruned and their unique chunks GC'd.
- **E2E with real SeaweedFS** (Docker, `--ignored` gate): boot the SeaweedFS container in `infra/`, configure as target, backup-restore-verify a real LocalFile-backed VM.
- **No regression**: every Plan 1/2/3 test continues to pass.

## Success criteria

- A LocalFile or TrueNasIscsi volume can be backed up to a SeaweedFS / MinIO / AWS-S3 target via `POST /v1/volumes/:id/backup`.
- The same backup, restored to a fresh volume, contains byte-identical bytes (golden test).
- Re-backing up the same volume after no changes uploads only the manifest (full dedup hit).
- Re-backing up after partial change uploads only the changed-data chunks (FastCDC delta).
- Backup-of-2-similar-VMs uses ~50% of the space of two unrelated full backups (cross-volume dedup).
- Manager restart mid-backup leaves a `running` row that auto-resumes via the reconciler within 5 minutes.
- DB destroyed → `nqrust backup index-rebuild` reconstructs all backup rows from S3 manifests.
- Daily GC reclaims chunks made unreachable by retention pruning, with no false positives (no chunk deleted while a manifest still references it).
- `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` pass.

## Glossary

- **Chunk** — a variable-size segment of the volume's bytes. FastCDC determines boundaries content-defined; chunk size targets ~64KB average.
- **Content-addressed** — chunks are stored under an S3 key derived from `BLAKE3(ciphertext)`. Identical content → identical key → S3 dedup is automatic.
- **Convergent encryption** — encryption with a deterministic nonce derived from the plaintext. Preserves dedup at the cost of weakening to a "confirmation of file" attack.
- **Envelope key** — the manager's master key, used to encrypt other keys (target keys, S3 secrets) at rest in the DB. Rotates via key-rotation procedure (out of scope for this spec).
- **Manifest** — per-backup metadata: ordered list of `ChunkRef { plaintext_offset, plaintext_length, chunk_id, ciphertext_length }`. Stored encrypted in S3 under `manifests/<backup-uuid>.bin` and referenced from `backup.manifest_object_key`.
- **Mark-and-sweep** — GC algorithm: read all live manifests (mark phase), list S3 objects (sweep phase), delete unmarked objects.
- **Skip-if-exists** — backup-time pattern: HEAD before PUT. If the chunk exists, skip the PUT. The dedup engine.
