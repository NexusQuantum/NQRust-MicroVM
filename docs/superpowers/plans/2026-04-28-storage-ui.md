# Storage UI Implementation Plan (Plan 3 of 3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Surface the storage backend abstraction in the user-facing Next.js UI. Add `BackendSelector` component to VM-create and volume-create forms; hide it when only one backend is configured. After this plan, operators can select a backend per VM/volume from the UI without any CLI/API hand-knowledge.

**Architecture:** Pure frontend work in `apps/ui`. New TypeScript types mirror the existing `StorageBackend` wire type. New `useStorageBackends()` React Query hook talks to `GET /v1/storage_backends`. New `BackendSelector` component renders a dropdown sourced from that hook; renders nothing (or disabled) when the response has only the `localfile-default` row. Both VM-create and volume-create forms get the selector.

**Tech Stack:** Next.js 15, React 19, TypeScript, TanStack Query, shadcn/ui (Radix Select), Tailwind CSS 4. No backend changes.

**Spec:** `docs/superpowers/specs/2026-04-28-storage-hci-design.md`. Builds on Plans 1 and 2. The API was added in Plan 1 (`GET /v1/storage_backends`).

---

## File structure

Additions:
- `apps/ui/lib/types/storage.ts` (or extend `apps/ui/lib/types/index.ts` if that's the convention)
- `apps/ui/components/storage/backend-selector.tsx`

Modifications:
- `apps/ui/lib/queries.ts` — add `useStorageBackends()` and key `["storage_backends"]`.
- `apps/ui/lib/api/facade.ts` — add `listStorageBackends()` calling `GET /v1/storage_backends`.
- `apps/ui/components/vm/vm-create-form.tsx` (the dialog/form for creating a VM) — embed `<BackendSelector />`; submit `backend_id` in the request body when set.
- `apps/ui/components/volume/volume-create-form.tsx` — same treatment.

Tests (best-effort; UI tests in this codebase are limited):
- Component-level smoke test for `BackendSelector` rendering and conditional hide. (Skip if no test infra; document.)

---

## Task 1: TypeScript types

**Files:**
- Modify: `apps/ui/lib/types/index.ts`

- [ ] **Step 1.1: Add types**

Append to `apps/ui/lib/types/index.ts`:

```ts
export type BackendKind = "local_file" | "iscsi" | "truenas_iscsi";

export interface Capabilities {
  supports_native_snapshots: boolean;
  supports_concurrent_attach: boolean;
  supports_live_migration: boolean;
  supports_clone_from_image: boolean;
}

export interface StorageBackend {
  id: string;          // UUID
  name: string;
  kind: BackendKind;
  capabilities: Capabilities;
  is_default: boolean;
  created_at: string;  // ISO 8601
  deleted_at?: string | null;
}

export interface StorageBackendListResponse {
  items: StorageBackend[];
}
```

- [ ] **Step 1.2: Verify** `(cd apps/ui && pnpm tsc --noEmit)` clean.

- [ ] **Step 1.3: Commit**

```bash
git add apps/ui/lib/types/index.ts
git commit -m "feat(storage): UI types for StorageBackend, BackendKind, Capabilities"
```

---

## Task 2: API facade method

**Files:**
- Modify: `apps/ui/lib/api/facade.ts`

- [ ] **Step 2.1: Add method**

Inside the FacadeApi class, add:

```ts
async listStorageBackends(): Promise<StorageBackendListResponse> {
  return this.client.get<StorageBackendListResponse>("/v1/storage_backends");
}
```

(Adjust the import block to include `StorageBackendListResponse` from the types file. Match the existing facade pattern — if it uses `this.http` not `this.client`, follow that.)

- [ ] **Step 2.2: Verify + commit**

```bash
git add apps/ui/lib/api/facade.ts
git commit -m "feat(storage): facade.listStorageBackends()"
```

---

## Task 3: React Query hook

**Files:**
- Modify: `apps/ui/lib/queries.ts`

- [ ] **Step 3.1: Add hook**

Append:

```ts
import type { StorageBackend, StorageBackendListResponse } from "./types";

const STORAGE_BACKENDS_KEY = ["storage_backends"] as const;

export function useStorageBackends() {
  return useQuery({
    queryKey: STORAGE_BACKENDS_KEY,
    queryFn: async (): Promise<StorageBackend[]> => {
      const resp = await api.listStorageBackends();
      return resp.items;
    },
    staleTime: 60_000, // backends rarely change at runtime
  });
}

export const queryKeys = {
  ...queryKeys,
  storageBackends: () => STORAGE_BACKENDS_KEY,
};
```

(Adjust to match the existing `queryKeys` object pattern in this file. The hook name and structure should mirror the existing `useTemplates()` / `useNetworks()` etc.)

- [ ] **Step 3.2: Verify + commit**

```bash
git add apps/ui/lib/queries.ts
git commit -m "feat(storage): useStorageBackends() React Query hook"
```

---

## Task 4: `BackendSelector` component

**Files:**
- Create: `apps/ui/components/storage/backend-selector.tsx`

- [ ] **Step 4.1: Component**

```tsx
// apps/ui/components/storage/backend-selector.tsx
"use client";

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Label } from "@/components/ui/label";
import { useStorageBackends } from "@/lib/queries";
import type { StorageBackend } from "@/lib/types";

export interface BackendSelectorProps {
  value: string | undefined;
  onChange: (backendId: string | undefined) => void;
  /** When the cluster has only one configured backend, the selector hides itself. */
  hideWhenSingle?: boolean;
  /** Optional id for the form label. */
  id?: string;
  label?: string;
}

export function BackendSelector({
  value,
  onChange,
  hideWhenSingle = true,
  id = "backend-selector",
  label = "Storage backend",
}: BackendSelectorProps) {
  const { data, isLoading, error } = useStorageBackends();

  if (isLoading) return null;
  if (error || !data) return null;

  const active = data.filter((b: StorageBackend) => !b.deleted_at);
  if (hideWhenSingle && active.length <= 1) {
    // Single (or zero) backend: don't clutter the form. Submit defaults via API.
    return null;
  }

  const defaultId = active.find((b) => b.is_default)?.id ?? active[0]?.id;
  const selectedId = value ?? defaultId;

  return (
    <div className="space-y-1.5">
      <Label htmlFor={id}>{label}</Label>
      <Select
        value={selectedId}
        onValueChange={(v) => onChange(v === defaultId ? undefined : v)}
      >
        <SelectTrigger id={id} className="w-full">
          <SelectValue placeholder="Default" />
        </SelectTrigger>
        <SelectContent>
          {active.map((b) => (
            <SelectItem key={b.id} value={b.id}>
              {b.name} {b.is_default ? "(default)" : ""}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
```

- [ ] **Step 4.2: Verify TypeScript** — `(cd apps/ui && pnpm tsc --noEmit)` clean.

- [ ] **Step 4.3: Commit**

```bash
git add apps/ui/components/storage/backend-selector.tsx
git commit -m "feat(storage): BackendSelector component (hides when single backend)"
```

---

## Task 5: Embed in VM-create form

**Files:**
- Modify: `apps/ui/components/vm/vm-create-form.tsx` (or wherever the VM-create form actually lives — could be `apps/ui/app/(dashboard)/vms/new/page.tsx` or a similar path; locate via `grep -rn "create.*VM\|CreateVM" apps/ui`)

- [ ] **Step 5.1: Locate the form**

```bash
grep -rn "createVm\|CreateVm\|create.*VM\|/vms/new" apps/ui --include "*.tsx" --include "*.ts" 2>&1 | head -20
```

Identify the form component file.

- [ ] **Step 5.2: Add state + selector**

Inside the form component:

```tsx
const [backendId, setBackendId] = useState<string | undefined>(undefined);
```

In the form JSX, near the storage-related fields (rootfs size, drive count, etc.), insert:

```tsx
<BackendSelector
  value={backendId}
  onChange={setBackendId}
/>
```

In the submit handler, include `backend_id: backendId` in the request body. If `backendId` is undefined, omit the field (the API treats absence as default).

- [ ] **Step 5.3: Verify + commit**

```bash
(cd apps/ui && pnpm tsc --noEmit && pnpm lint)
git add apps/ui/components/vm/ apps/ui/app/
git commit -m "feat(storage): VM-create form supports backend_id selection"
```

---

## Task 6: Embed in volume-create form

**Files:** `apps/ui/components/volume/volume-create-form.tsx` (or similar)

- [ ] **Step 6.1: Locate the form**

```bash
grep -rn "createVolume\|CreateVolume\|/volumes/new" apps/ui --include "*.tsx" --include "*.ts" 2>&1 | head -10
```

- [ ] **Step 6.2: Same pattern as Task 5**

Add state, embed `<BackendSelector />`, include `backend_id` in submit body.

- [ ] **Step 6.3: Verify + commit**

```bash
(cd apps/ui && pnpm tsc --noEmit && pnpm lint)
git add apps/ui/components/volume/ apps/ui/app/
git commit -m "feat(storage): volume-create form supports backend_id selection"
```

---

## Task 7: Final sweep

- [ ] `(cd apps/ui && pnpm tsc --noEmit)` — clean
- [ ] `(cd apps/ui && pnpm lint)` — clean
- [ ] `(cd apps/ui && pnpm build)` — succeeds (catches NextJS-side regressions)
- [ ] Manual smoke test: start manager + UI; with only `localfile-default` configured, verify the BackendSelector is hidden in both forms. Add a second backend via TOML, restart manager, verify the selector appears with both options.
- [ ] Commit any fmt fixes.

---

## Plan 3 completion checklist

- [ ] `apps/ui/lib/types/index.ts` exports `StorageBackend`, `BackendKind`, `Capabilities`
- [ ] `useStorageBackends()` hook returns active backends, refetches on stale-time
- [ ] `BackendSelector` renders a dropdown when ≥2 active backends; renders null when ≤1
- [ ] VM-create and volume-create forms send `backend_id` in the request body when set
- [ ] `pnpm tsc --noEmit && pnpm lint && pnpm build` clean
- [ ] No backend code changes (this plan is UI-only)

## Out of scope

- Backend administration UI (create/edit/delete from UI). Backends are managed via TOML in this PR.
- Capability-aware UI hints (e.g., disabling "snapshot" button for non-snapshot backends). Will be added later as features rely on them.
- Per-volume backend display in the volume list (just shows the resolved name; no UI yet for capability badges).
