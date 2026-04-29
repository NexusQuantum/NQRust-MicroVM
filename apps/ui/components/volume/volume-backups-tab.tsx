"use client";
import { BackupList } from "@/components/backup/backup-list";
import { BackupScheduleEditor } from "@/components/backup/backup-schedule-editor";
import { useBackupTargets } from "@/lib/queries";
import { facadeApi } from "@/lib/api/facade";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { useState } from "react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Label } from "@/components/ui/label";

export function VolumeBackupsTab({ volumeId }: { volumeId: string }) {
  const { data: targets } = useBackupTargets();
  const [target, setTarget] = useState<string | undefined>();
  const qc = useQueryClient();
  const back = useMutation({
    mutationFn: () => facadeApi.createBackup(volumeId, target!),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["backups", volumeId] }),
  });
  return (
    <div className="space-y-6">
      <section>
        <h2 className="text-lg font-semibold mb-2">Back up now</h2>
        <div className="flex gap-2 items-end">
          <div className="flex-1">
            <Label>Target</Label>
            <Select value={target} onValueChange={setTarget}>
              <SelectTrigger>
                <SelectValue placeholder="Select…" />
              </SelectTrigger>
              <SelectContent>
                {(targets ?? []).map((t) => (
                  <SelectItem key={t.id} value={t.id}>
                    {t.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <Button disabled={!target || back.isPending} onClick={() => back.mutate()}>
            {back.isPending ? "Starting…" : "Backup now"}
          </Button>
        </div>
      </section>
      <section>
        <h2 className="text-lg font-semibold mb-2">Schedule</h2>
        <BackupScheduleEditor volumeId={volumeId} />
      </section>
      <section>
        <h2 className="text-lg font-semibold mb-2">History</h2>
        <BackupList volumeId={volumeId} />
      </section>
    </div>
  );
}
