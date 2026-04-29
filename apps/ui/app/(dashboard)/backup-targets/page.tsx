"use client";
import { useBackupTargets } from "@/lib/queries";
import { BackupTargetForm } from "@/components/backup/backup-target-form";

export default function BackupTargetsPage() {
  const { data: targets, isLoading } = useBackupTargets();
  return (
    <div className="space-y-6 p-4">
      <h1 className="text-2xl font-bold">Backup targets</h1>
      <BackupTargetForm />
      <div>
        <h2 className="text-lg font-semibold">Configured targets</h2>
        {isLoading && <p>Loading…</p>}
        <ul className="space-y-1">
          {(targets ?? []).map((t) => (
            <li key={t.id} className="border p-2 rounded">
              <div className="font-medium">{t.name}</div>
              <div className="text-sm text-muted-foreground">
                {t.endpoint} → s3://{t.bucket}/{t.prefix}
              </div>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
