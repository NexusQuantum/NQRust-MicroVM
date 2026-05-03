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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import type { BackendKind } from "@/lib/types";
import { useCreateStorageBackend } from "@/lib/queries";

interface Field {
  key: string;
  label: string;
  placeholder?: string;
  hint?: string;
  type?: "text" | "password";
}

/** Per-kind config fields. Keep aligned with `apps/manager/src/features/storage/config.rs::validate`. */
const KIND_CONFIG: Record<BackendKind, { label: string; description: string; fields: Field[] }> = {
  local_file: {
    label: "Local file",
    description: "VM disks live as files on the manager's local filesystem. No external dependencies.",
    fields: [
      {
        key: "root_dir",
        label: "Root directory",
        placeholder: "/srv/fc/vms",
        hint: "Directory where the manager creates VM disk files. Must be writable by the manager process.",
      },
    ],
  },
  nfs: {
    label: "NFS",
    description:
      "VM disks live as files on an NFS export. The manager must have the export mounted at manager_mount_path; agents mount on demand.",
    fields: [
      { key: "server", label: "NFS server", placeholder: "10.0.0.5", hint: "IP or hostname of the NFS server." },
      { key: "export", label: "Export path", placeholder: "/mnt/tank/vms", hint: "Path the server exports." },
      {
        key: "manager_mount_path",
        label: "Manager mount path",
        placeholder: "/mnt/nfs-mgr",
        hint: "Where the manager process has the export mounted locally. Must exist and be writable.",
      },
    ],
  },
  iscsi: {
    label: "iSCSI (generic)",
    description:
      "Operator-managed iSCSI target. Provisioning is no-op — LUNs are pre-created on the target and registered via API.",
    fields: [
      {
        key: "target_iqn",
        label: "Target IQN",
        placeholder: "iqn.2024-01.com.example:storage",
        hint: "iSCSI Qualified Name of the target.",
      },
      {
        key: "portal",
        label: "Portal (optional)",
        placeholder: "10.0.0.5:3260",
        hint: "Discovery portal — used by the agent for iscsiadm login.",
      },
    ],
  },
  truenas_iscsi: {
    label: "TrueNAS (iSCSI)",
    description: "TrueNAS REST API provisions zvols + extents + targets automatically.",
    fields: [
      {
        key: "endpoint",
        label: "TrueNAS endpoint",
        placeholder: "https://truenas.lan",
        hint: "Base URL of the TrueNAS REST API.",
      },
      {
        key: "api_key_env",
        label: "API key env var name",
        placeholder: "TRUENAS_API_KEY",
        hint: "Name of the environment variable on the manager process that holds the API key. Don't paste the key itself here.",
      },
      { key: "pool", label: "Pool", placeholder: "tank", hint: "ZFS pool that hosts the test zvols." },
      {
        key: "target_iqn_prefix",
        label: "Target IQN prefix",
        placeholder: "iqn.2024-01.com.example",
        hint: "TrueNAS appends a per-LUN suffix; this is the static prefix.",
      },
    ],
  },
  spdk_lvol: {
    label: "SPDK lvol",
    description: "Single-node SPDK lvol backend. Requires SPDK running locally.",
    fields: [
      {
        key: "rpc_socket",
        label: "RPC socket",
        placeholder: "/var/tmp/spdk.sock",
        hint: "SPDK JSON-RPC Unix socket path.",
      },
      { key: "lvs_name", label: "LVS name", placeholder: "lvs0", hint: "Name of the SPDK logical volume store." },
    ],
  },
};

const KINDS: BackendKind[] = ["local_file", "nfs", "iscsi", "truenas_iscsi", "spdk_lvol"];

interface Props {
  open: boolean;
  onOpenChange: (v: boolean) => void;
}

export function BackendCreateDialog({ open, onOpenChange }: Props) {
  const create = useCreateStorageBackend();
  const [name, setName] = useState("");
  const [kind, setKind] = useState<BackendKind>("nfs");
  const [isDefault, setIsDefault] = useState(false);
  const [config, setConfig] = useState<Record<string, string>>({});

  const spec = KIND_CONFIG[kind];

  function reset() {
    setName("");
    setKind("nfs");
    setIsDefault(false);
    setConfig({});
  }

  async function submit() {
    const cleaned: Record<string, string> = {};
    for (const f of spec.fields) {
      const v = (config[f.key] ?? "").trim();
      if (!v && !/(optional)/i.test(f.label)) {
        // Server-side validate() will reject; surface a friendlier first
        // pass here by skipping submit until required fields are filled.
        return;
      }
      if (v) cleaned[f.key] = v;
    }
    await create.mutateAsync({
      name: name.trim(),
      kind,
      is_default: isDefault,
      config: cleaned,
    });
    if (!create.isError) {
      reset();
      onOpenChange(false);
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        onOpenChange(o);
        if (!o) reset();
      }}
    >
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Add storage backend</DialogTitle>
          <DialogDescription>
            Configure a place where VM disks can live. External backends (NFS, iSCSI, TrueNAS) need
            the network/credentials below.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="bk-name">Name</Label>
            <Input
              id="bk-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. truenas-prod"
            />
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="bk-kind">Kind</Label>
            <Select value={kind} onValueChange={(v) => { setKind(v as BackendKind); setConfig({}); }}>
              <SelectTrigger id="bk-kind">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {KINDS.map((k) => (
                  <SelectItem key={k} value={k}>
                    {KIND_CONFIG[k].label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">{spec.description}</p>
          </div>

          {spec.fields.map((f) => (
            <div key={f.key} className="space-y-1.5">
              <Label htmlFor={`bk-cfg-${f.key}`}>{f.label}</Label>
              <Input
                id={`bk-cfg-${f.key}`}
                type={f.type ?? "text"}
                value={config[f.key] ?? ""}
                onChange={(e) => setConfig({ ...config, [f.key]: e.target.value })}
                placeholder={f.placeholder}
              />
              {f.hint && <p className="text-xs text-muted-foreground">{f.hint}</p>}
            </div>
          ))}

          <div className="flex items-center justify-between rounded-md border p-3">
            <div className="space-y-0.5">
              <Label htmlFor="bk-default" className="cursor-pointer">
                Set as default
              </Label>
              <p className="text-xs text-muted-foreground">
                New VMs use the default backend when none is selected.
              </p>
            </div>
            <Switch id="bk-default" checked={isDefault} onCheckedChange={setIsDefault} />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={create.isPending}>
            Cancel
          </Button>
          <Button onClick={submit} disabled={!name.trim() || create.isPending}>
            {create.isPending ? "Adding..." : "Add backend"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
