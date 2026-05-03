"use client";

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
import { Trash2 } from "lucide-react";
import type { BackendKind, StorageBackend } from "@/lib/types";
import { useDeleteStorageBackend } from "@/lib/queries";

const KIND_LABEL: Record<BackendKind, string> = {
  local_file: "Local file",
  iscsi: "iSCSI (generic)",
  truenas_iscsi: "TrueNAS (iSCSI)",
  spdk_lvol: "SPDK lvol",
  nfs: "NFS",
};

const EXTERNAL_KINDS: BackendKind[] = ["iscsi", "truenas_iscsi", "nfs"];

export function BackendTable({ backends }: { backends: StorageBackend[] }) {
  const del = useDeleteStorageBackend();

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Name</TableHead>
          <TableHead>Kind</TableHead>
          <TableHead>Capabilities</TableHead>
          <TableHead>Default</TableHead>
          <TableHead className="text-right">Actions</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {backends.map((b) => (
          <TableRow key={b.id}>
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
            <TableCell>
              {b.is_default ? <Badge>default</Badge> : <span className="text-muted-foreground">—</span>}
            </TableCell>
            <TableCell className="text-right">
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
  );
}
