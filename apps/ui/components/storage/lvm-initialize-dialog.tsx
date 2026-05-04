"use client";

import { useState } from "react";
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
import { useInitializeBackend } from "@/lib/queries";

interface Props {
  /** Backend id to initialize. When null, the dialog renders nothing
   *  meaningful — guarded so the post-create banner can pass the id it
   *  just created without having to lazy-mount the component. */
  backendId: string | null;
  backendName: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/** The exact phrase the manager requires in the request body. The
 *  manager rejects anything else with HTTP 400 — keep this string
 *  byte-identical to `apps/manager/src/features/storage/routes.rs`. */
const REQUIRED_CONFIRM = "I understand this wipes the LUN";

/** Destructive type-to-confirm dialog that runs the one-time
 *  pvcreate + vgcreate against the freshly-attached iSCSI LUN. The
 *  operator has to type the required phrase verbatim before the
 *  Initialize button enables. */
export function LvmInitializeDialog({
  backendId,
  backendName,
  open,
  onOpenChange,
}: Props) {
  const [typed, setTyped] = useState("");
  const matches = typed === REQUIRED_CONFIRM;
  const init = useInitializeBackend();

  const handleConfirm = async () => {
    if (!backendId || !matches) return;
    try {
      await init.mutateAsync({ id: backendId, confirm: typed });
      setTyped("");
      onOpenChange(false);
    } catch {
      // The mutation surfaces the error via init.error and a toast in
      // the hook — keep the dialog open so the operator can read it.
    }
  };

  const handleOpenChange = (next: boolean) => {
    // Reset on close so a second invocation starts clean.
    if (!next) {
      setTyped("");
      init.reset();
    }
    onOpenChange(next);
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Initialize Volume Group on {backendName}</DialogTitle>
          <DialogDescription>
            This is a one-time, destructive operation.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="rounded-md border border-red-300 bg-red-50 p-3 text-sm dark:border-red-700 dark:bg-red-950/40">
            <p className="font-medium">Warning — this wipes the LUN.</p>
            <ul className="mt-2 list-disc space-y-1 pl-5 text-xs">
              <li>
                The agent runs <code className="font-mono">pvcreate</code> followed by{" "}
                <code className="font-mono">vgcreate</code> on the shared iSCSI LUN.
              </li>
              <li>
                Any non-LVM data already on the LUN will be erased. If the LUN
                already holds a different filesystem or a different VG, that
                data is unrecoverable after this step.
              </li>
              <li>
                Run this once per backend, while no VMs are using the LUN.
              </li>
            </ul>
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="lvm-init-confirm">
              Type <code className="font-mono text-xs">{REQUIRED_CONFIRM}</code> to confirm
            </Label>
            <Input
              id="lvm-init-confirm"
              value={typed}
              onChange={(e) => setTyped(e.target.value)}
              placeholder={REQUIRED_CONFIRM}
              autoComplete="off"
              spellCheck={false}
              disabled={init.isPending}
            />
          </div>

          {init.error && (
            <div className="rounded-md border border-red-300 bg-red-50 p-3 text-xs text-red-700 dark:border-red-700 dark:bg-red-950/40 dark:text-red-300">
              {init.error instanceof Error ? init.error.message : String(init.error)}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => handleOpenChange(false)}
            disabled={init.isPending}
          >
            Cancel
          </Button>
          <Button
            variant="destructive"
            onClick={handleConfirm}
            disabled={!backendId || !matches || init.isPending}
          >
            {init.isPending ? "Initializing..." : "Initialize"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
