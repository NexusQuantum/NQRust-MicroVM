"use client";
import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { facadeApi } from "@/lib/api/facade";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

export function BackupTargetForm({ onCreated }: { onCreated?: () => void }) {
  const [name, setName] = useState("");
  const [endpoint, setEndpoint] = useState("");
  const [bucket, setBucket] = useState("");
  const [prefix, setPrefix] = useState("");
  const [accessKey, setAccessKey] = useState("");
  const [secretKey, setSecretKey] = useState("");
  const [region, setRegion] = useState("us-east-1");
  const qc = useQueryClient();
  const mut = useMutation({
    mutationFn: () =>
      facadeApi.createBackupTarget({
        name,
        endpoint,
        bucket,
        prefix,
        access_key_id: accessKey,
        secret_access_key: secretKey,
        region,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["backup_targets"] });
      onCreated?.();
    },
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
        <Label>Name</Label>
        <Input value={name} onChange={(e) => setName(e.target.value)} required />
      </div>
      <div>
        <Label>Endpoint URL</Label>
        <Input
          value={endpoint}
          onChange={(e) => setEndpoint(e.target.value)}
          required
          placeholder="https://seaweedfs.local:8333"
        />
      </div>
      <div>
        <Label>Region</Label>
        <Input value={region} onChange={(e) => setRegion(e.target.value)} />
      </div>
      <div>
        <Label>Bucket</Label>
        <Input value={bucket} onChange={(e) => setBucket(e.target.value)} required />
      </div>
      <div>
        <Label>Prefix (optional)</Label>
        <Input value={prefix} onChange={(e) => setPrefix(e.target.value)} />
      </div>
      <div>
        <Label>Access Key ID</Label>
        <Input value={accessKey} onChange={(e) => setAccessKey(e.target.value)} required />
      </div>
      <div>
        <Label>Secret Access Key</Label>
        <Input
          type="password"
          value={secretKey}
          onChange={(e) => setSecretKey(e.target.value)}
          required
        />
      </div>
      <Button type="submit" disabled={mut.isPending}>
        {mut.isPending ? "Saving…" : "Create target"}
      </Button>
      {mut.error && (
        <p className="text-red-500 text-sm">{(mut.error as Error).message}</p>
      )}
    </form>
  );
}
