"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import type { StorageBackend, BackendKind } from "@/lib/types";
import { useBackendConfig, useUpdateStorageBackend } from "@/lib/queries";

interface Props {
  backend: StorageBackend | null;
  open: boolean;
  onOpenChange: (v: boolean) => void;
}

export function BackendEditDialog({ backend, open, onOpenChange }: Props) {
  const update = useUpdateStorageBackend();
  const { data: configResp, isLoading: configLoading } = useBackendConfig(
    backend?.id ?? null,
  );
  const [isDefault, setIsDefault] = useState(false);

  useEffect(() => {
    if (backend) {
      setIsDefault(backend.is_default);
    }
  }, [backend]);

  if (!backend) return null;

  async function submit() {
    if (!backend || !configResp) return;
    await update.mutateAsync({
      id: backend.id,
      req: {
        name: backend.name,
        kind: backend.kind as BackendKind,
        is_default: isDefault,
        // Round-trip the existing config so manager-side validate()
        // passes. Editing kind-specific fields lives in a future
        // version that surfaces them as form inputs here; for v1 the
        // only knob the operator can change is is_default.
        config: configResp.config,
      },
    });
    if (!update.isError) onOpenChange(false);
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Edit {backend.name}</DialogTitle>
          <DialogDescription>
            Toggle the default flag for this backend. Editing kind-specific
            config (URLs, keys, mount paths) is a follow-up — Remove and
            re-Add for now.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4">
          <div className="flex items-center justify-between rounded-md border p-3">
            <div className="space-y-0.5">
              <Label htmlFor="be-default" className="cursor-pointer">
                Set as default
              </Label>
            </div>
            <Switch id="be-default" checked={isDefault} onCheckedChange={setIsDefault} />
          </div>
          {configLoading && (
            <p className="text-xs text-muted-foreground">Loading existing config…</p>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={update.isPending}>
            Cancel
          </Button>
          <Button
            onClick={submit}
            disabled={update.isPending || configLoading || !configResp}
          >
            {update.isPending ? "Saving..." : "Save"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
