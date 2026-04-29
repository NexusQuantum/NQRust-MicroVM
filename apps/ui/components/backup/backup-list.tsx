"use client";
import { useBackups } from "@/lib/queries";
import { facadeApi } from "@/lib/api/facade";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { useState } from "react";
import { RestoreDialog } from "./restore-dialog";

export function BackupList({ volumeId }: { volumeId: string }) {
  const { data: backups, isLoading } = useBackups(volumeId);
  const qc = useQueryClient();
  const del = useMutation({
    mutationFn: (id: string) => facadeApi.deleteBackup(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["backups", volumeId] }),
  });
  const [restoring, setRestoring] = useState<string | null>(null);

  if (isLoading) return <p>Loading…</p>;
  if (!backups?.length) return <p className="text-muted-foreground">No backups yet.</p>;

  return (
    <>
      <table className="w-full text-sm">
        <thead className="text-left text-muted-foreground">
          <tr>
            <th>Created</th>
            <th>Status</th>
            <th>Size</th>
            <th>Chunks</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {backups.map((b) => (
            <tr key={b.id} className="border-t">
              <td>{new Date(b.created_at).toLocaleString()}</td>
              <td>{b.status}</td>
              <td>{(b.size_bytes / 1024 / 1024).toFixed(1)} MiB</td>
              <td>{b.chunk_count}</td>
              <td className="text-right space-x-2">
                {b.status === "completed" && (
                  <Button size="sm" variant="secondary" onClick={() => setRestoring(b.id)}>
                    Restore…
                  </Button>
                )}
                <Button size="sm" variant="ghost" onClick={() => del.mutate(b.id)}>
                  Delete
                </Button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {restoring && <RestoreDialog backupId={restoring} onClose={() => setRestoring(null)} />}
    </>
  );
}
