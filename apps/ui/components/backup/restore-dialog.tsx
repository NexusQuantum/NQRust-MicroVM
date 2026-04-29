"use client";
import { useState } from "react";
import { useStorageBackends } from "@/lib/queries";
import { facadeApi } from "@/lib/api/facade";
import { useMutation } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Label } from "@/components/ui/label";

export function RestoreDialog({
  backupId,
  onClose,
}: {
  backupId: string;
  onClose: () => void;
}) {
  const { data: backends } = useStorageBackends();
  const active = (backends ?? []).filter((b) => !b.deleted_at);
  const [target, setTarget] = useState<string | undefined>(
    active.find((b) => b.is_default)?.id,
  );
  const mut = useMutation({
    mutationFn: () => facadeApi.restoreBackup(backupId, target!),
    onSuccess: () => onClose(),
  });
  return (
    <Dialog open onOpenChange={(o) => !o && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Restore backup to a new volume</DialogTitle>
        </DialogHeader>
        <div className="space-y-2">
          <Label>Target backend</Label>
          <Select value={target} onValueChange={setTarget}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {active.map((b) => (
                <SelectItem key={b.id} value={b.id}>
                  {b.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button disabled={!target || mut.isPending} onClick={() => mut.mutate()}>
            {mut.isPending ? "Restoring…" : "Restore"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
