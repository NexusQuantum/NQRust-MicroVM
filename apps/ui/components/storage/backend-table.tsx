"use client";

import { useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Pencil, Trash2, Wrench } from "lucide-react";
import type { BackendKind, StorageBackend } from "@/lib/types";
import { useDeleteStorageBackend, useBackendHealth } from "@/lib/queries";
import { BackendEditDialog } from "@/components/storage/backend-edit-dialog";
import { LvmInitializeDialog } from "@/components/storage/lvm-initialize-dialog";

const KIND_LABEL: Record<BackendKind, string> = {
  local_file: "Local file",
  iscsi: "iSCSI (generic)",
  truenas_iscsi: "TrueNAS (iSCSI)",
  spdk_lvol: "SPDK lvol",
  nfs: "NFS",
  iscsi_lvm: "iSCSI + LVM",
};

const EXTERNAL_KINDS: BackendKind[] = ["iscsi", "truenas_iscsi", "nfs", "iscsi_lvm"];

function StatusDot({ id }: { id: string }) {
  const { data, isLoading } = useBackendHealth(id);
  if (isLoading || !data) {
    return <span className="inline-block h-2.5 w-2.5 rounded-full bg-muted" />;
  }
  return (
    <span
      className={`inline-block h-2.5 w-2.5 rounded-full ${
        data.reachable ? "bg-green-500" : "bg-red-500"
      }`}
      title={data.status}
    />
  );
}

function CapacityCell({ id }: { id: string }) {
  const { data } = useBackendHealth(id);
  if (!data || data.total_bytes === undefined || data.used_bytes === undefined) {
    return <span className="text-muted-foreground">—</span>;
  }
  const used = formatBytes(data.used_bytes);
  const total = formatBytes(data.total_bytes);
  const pct = data.total_bytes > 0 ? Math.round((data.used_bytes / data.total_bytes) * 100) : 0;
  return (
    <span className="text-xs">
      {used} / {total}{" "}
      <span className="text-muted-foreground">({pct}%)</span>
    </span>
  );
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KiB`;
  if (n < 1024 ** 3) return `${(n / 1024 ** 2).toFixed(1)} MiB`;
  if (n < 1024 ** 4) return `${(n / 1024 ** 3).toFixed(2)} GiB`;
  return `${(n / 1024 ** 4).toFixed(2)} TiB`;
}

/** Show an "Initialize" wrench button on iscsi_lvm rows whose VG hasn't
 *  been created yet. Health probe returns total_bytes only after a
 *  vgcreate succeeds — so undefined / 0 means the VG is not ready. */
function InitializeButton({
  backend,
  onClick,
}: {
  backend: StorageBackend;
  onClick: () => void;
}) {
  const { data } = useBackendHealth(backend.id);
  if (backend.kind !== "iscsi_lvm") return null;
  const initialized = (data?.total_bytes ?? 0) > 0;
  if (initialized) return null;
  return (
    <Button
      variant="ghost"
      size="icon"
      aria-label={`Initialize ${backend.name}`}
      title="Initialize volume group (one-time, destructive)"
      onClick={onClick}
    >
      <Wrench className="h-4 w-4" />
    </Button>
  );
}

export function BackendTable({ backends }: { backends: StorageBackend[] }) {
  const del = useDeleteStorageBackend();
  const [editing, setEditing] = useState<StorageBackend | null>(null);
  const [initializing, setInitializing] = useState<StorageBackend | null>(null);

  return (
    <>
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead className="w-[50px]">Status</TableHead>
          <TableHead>Name</TableHead>
          <TableHead>Kind</TableHead>
          <TableHead>Capabilities</TableHead>
          <TableHead>Capacity</TableHead>
          <TableHead>Default</TableHead>
          <TableHead className="text-right">Actions</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {backends.map((b) => (
          <TableRow key={b.id}>
            <TableCell><StatusDot id={b.id} /></TableCell>
            <TableCell className="font-medium">{b.name}</TableCell>
            <TableCell>
              <div className="flex items-center gap-2">
                <Badge variant="outline">{KIND_LABEL[b.kind] ?? b.kind}</Badge>
                {EXTERNAL_KINDS.includes(b.kind) && (
                  <Badge variant="secondary">external</Badge>
                )}
              </div>
            </TableCell>
            <TableCell>
              <div className="flex flex-wrap gap-1 text-xs">
                {b.capabilities.supports_native_snapshots && (
                  <Badge variant="outline">snapshots</Badge>
                )}
                {b.capabilities.supports_clone_from_image && (
                  <Badge variant="outline">clone-from-image</Badge>
                )}
                {b.capabilities.supports_concurrent_attach && (
                  <Badge variant="outline">concurrent-attach</Badge>
                )}
                {b.capabilities.supports_live_migration && (
                  <Badge variant="outline">live-migration</Badge>
                )}
              </div>
            </TableCell>
            <TableCell><CapacityCell id={b.id} /></TableCell>
            <TableCell>
              {b.is_default ? <Badge>default</Badge> : <span className="text-muted-foreground">—</span>}
            </TableCell>
            <TableCell className="text-right">
              <InitializeButton backend={b} onClick={() => setInitializing(b)} />
              <Button
                variant="ghost"
                size="icon"
                aria-label={`Edit ${b.name}`}
                onClick={() => setEditing(b)}
              >
                <Pencil className="h-4 w-4" />
              </Button>
              <AlertDialog>
                <AlertDialogTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    aria-label={`Remove ${b.name}`}
                    disabled={b.is_default || del.isPending}
                    title={b.is_default ? "Default backend cannot be removed" : "Remove backend"}
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </AlertDialogTrigger>
                <AlertDialogContent>
                  <AlertDialogHeader>
                    <AlertDialogTitle>Remove {b.name}?</AlertDialogTitle>
                    <AlertDialogDescription>
                      The backend is soft-deleted. Volumes already provisioned on it
                      keep working, but no new VMs can use this backend until it is
                      re-added. The remove will fail if any live volumes still
                      reference it.
                    </AlertDialogDescription>
                  </AlertDialogHeader>
                  <AlertDialogFooter>
                    <AlertDialogCancel>Cancel</AlertDialogCancel>
                    <AlertDialogAction onClick={() => del.mutate(b.id)}>
                      Remove
                    </AlertDialogAction>
                  </AlertDialogFooter>
                </AlertDialogContent>
              </AlertDialog>
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
    <BackendEditDialog
      backend={editing}
      open={editing !== null}
      onOpenChange={(open) => {
        if (!open) setEditing(null);
      }}
    />
    <LvmInitializeDialog
      backendId={initializing?.id ?? null}
      backendName={initializing?.name ?? ""}
      open={initializing !== null}
      onOpenChange={(open) => {
        if (!open) setInitializing(null);
      }}
    />
    </>
  );
}
