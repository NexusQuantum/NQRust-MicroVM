"use client";

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Label } from "@/components/ui/label";
import { useStorageBackends } from "@/lib/queries";
import type { StorageBackend } from "@/lib/types";

export interface BackendSelectorProps {
  value: string | undefined;
  onChange: (backendId: string | undefined) => void;
  hideWhenSingle?: boolean;
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
              {b.name}
              {b.is_default ? " (default)" : ""}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
