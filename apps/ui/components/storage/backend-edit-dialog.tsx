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
import { Input } from "@/components/ui/input";
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
  const [newPassword, setNewPassword] = useState("");

  useEffect(() => {
    if (backend) {
      setIsDefault(backend.is_default);
      setNewPassword("");
    }
  }, [backend]);

  if (!backend) return null;

  async function submit() {
    if (!backend || !configResp) return;
    const isSmb = backend.kind === "smb";
    const trimmedPassword = newPassword.trim();
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
        // SMB-only: rotate the agent-side credential when the operator
        // typed a new password. Empty string ⇒ omit so the existing
        // credential stays active.
        ...(isSmb && trimmedPassword.length > 0
          ? { password: trimmedPassword }
          : {}),
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
          {backend?.kind === "smb" && (
            <div className="space-y-2 rounded-md border border-amber-500/30 bg-amber-500/5 p-3">
              <Label htmlFor="new-password">Rotate SMB password</Label>
              <Input
                id="new-password"
                type="password"
                autoComplete="new-password"
                placeholder="Leave blank to keep current"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                A non-empty value rotates the credential on the agent. The current password remains active if you leave this field blank.
              </p>
            </div>
          )}
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
