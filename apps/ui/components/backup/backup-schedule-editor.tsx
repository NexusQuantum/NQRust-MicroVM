"use client";
import { useState } from "react";
import { facadeApi } from "@/lib/api/facade";
import { useBackupTargets } from "@/lib/queries";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

export function BackupScheduleEditor({
  volumeId,
  current,
}: {
  volumeId: string;
  current?: { cron?: string; retain_count?: number; target_id?: string };
}) {
  const { data: targets } = useBackupTargets();
  const [cron, setCron] = useState(current?.cron ?? "0 2 * * *");
  const [retain, setRetain] = useState(current?.retain_count ?? 7);
  const [target, setTarget] = useState(current?.target_id);
  const qc = useQueryClient();
  const mut = useMutation({
    mutationFn: () =>
      facadeApi.patchBackupSchedule(volumeId, {
        cron,
        retain_count: retain,
        target_id: target,
      }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["volumes", volumeId] }),
  });
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        mut.mutate();
      }}
      className="space-y-3 max-w-md"
    >
      <div>
        <Label>Schedule (cron, UTC)</Label>
        <Input
          value={cron}
          onChange={(e) => setCron(e.target.value)}
          placeholder="0 2 * * *"
        />
      </div>
      <div>
        <Label>Retain count</Label>
        <Input
          type="number"
          min={1}
          value={retain}
          onChange={(e) => setRetain(parseInt(e.target.value))}
        />
      </div>
      <div>
        <Label>Backup target</Label>
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
      <Button type="submit" disabled={!target || mut.isPending}>
        {mut.isPending ? "Saving…" : "Save schedule"}
      </Button>
    </form>
  );
}
