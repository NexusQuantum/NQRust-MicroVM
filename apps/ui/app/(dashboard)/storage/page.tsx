"use client"

// B-III Task 1 UI replication panel.
//
// One page surfaces every read-only piece of the replication state so an
// operator can answer "where does my data live and is it healthy?"
// without reading agent logs or running curl. Mutating actions (repair,
// decommission, hot-spare toggle, plan execute) are surfaced as buttons
// with confirmation dialogs.

import { useMemo, useState } from "react"
import {
  useStorageBackends,
  useRaftGroups,
  useRaftGroupStatus,
  useRaftRepairQueue,
  useRebalancePlan,
  useExecutePlan,
  useRepairReplica,
  useSetHostHotSpare,
  useDecommissionHost,
  useHosts,
} from "@/lib/queries"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Switch } from "@/components/ui/switch"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { AlertCircle, CheckCircle2, Loader2, RefreshCw, ShieldAlert } from "lucide-react"

export default function StorageReplicationPage() {
  const backends = useStorageBackends()
  const raftBackends = useMemo(
    () => (backends.data ?? []).filter((b) => b.kind === "raft_spdk"),
    [backends.data]
  )
  const [selectedBackend, setSelectedBackend] = useState<string | undefined>(undefined)

  const activeBackend = selectedBackend ?? raftBackends[0]?.id

  if (backends.isLoading) {
    return (
      <div className="flex items-center gap-2 p-6 text-muted-foreground">
        <Loader2 className="h-4 w-4 animate-spin" />
        Loading storage backends…
      </div>
    )
  }

  if (raftBackends.length === 0) {
    return (
      <div className="p-6 space-y-4">
        <h1 className="text-2xl font-semibold">Replication</h1>
        <Card>
          <CardHeader>
            <CardTitle>No replicated backends configured</CardTitle>
            <CardDescription>
              Configure a <code className="rounded bg-muted px-1 py-0.5">raft_spdk</code> backend
              in your manager TOML and restart the manager. This page surfaces per-group
              membership, lagging followers, the repair queue, and operator actions
              (decommission, hot-spare promotion, rebalance) once at least one
              <code className="rounded bg-muted px-1 py-0.5">raft_spdk</code> backend is active.
            </CardDescription>
          </CardHeader>
        </Card>
      </div>
    )
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">Replication</h1>
          <p className="text-sm text-muted-foreground">
            Per-group membership, repair queue, and host lifecycle for raft_spdk backends.
          </p>
        </div>
        <Select value={activeBackend} onValueChange={setSelectedBackend}>
          <SelectTrigger className="w-[280px]">
            <SelectValue placeholder="Select a raft_spdk backend" />
          </SelectTrigger>
          <SelectContent>
            {raftBackends.map((b) => (
              <SelectItem key={b.id} value={b.id}>
                {b.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {activeBackend && (
        <Tabs defaultValue="groups" className="w-full">
          <TabsList>
            <TabsTrigger value="groups">Groups</TabsTrigger>
            <TabsTrigger value="hosts">Hosts</TabsTrigger>
            <TabsTrigger value="repair">Repair queue</TabsTrigger>
            <TabsTrigger value="rebalance">Rebalance</TabsTrigger>
          </TabsList>
          <TabsContent value="groups">
            <GroupsTab backendId={activeBackend} />
          </TabsContent>
          <TabsContent value="hosts">
            <HostsTab backendId={activeBackend} />
          </TabsContent>
          <TabsContent value="repair">
            <RepairQueueTab backendId={activeBackend} />
          </TabsContent>
          <TabsContent value="rebalance">
            <RebalanceTab backendId={activeBackend} />
          </TabsContent>
        </Tabs>
      )}
    </div>
  )
}

function GroupsTab({ backendId }: { backendId: string }) {
  const groups = useRaftGroups(backendId)
  const [selected, setSelected] = useState<string | undefined>()

  if (groups.isLoading) {
    return <Loader />
  }
  if (groups.isError) {
    return <ErrorBox label="groups">{(groups.error as Error)?.message}</ErrorBox>
  }
  const items = groups.data ?? []
  const activeGroup = selected ?? items[0]?.group_id

  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 mt-4">
      <Card className="lg:col-span-1">
        <CardHeader>
          <CardTitle>Groups</CardTitle>
          <CardDescription>{items.length} group(s) in this backend</CardDescription>
        </CardHeader>
        <CardContent className="p-0 max-h-[600px] overflow-auto">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Group</TableHead>
                <TableHead>Replicas</TableHead>
                <TableHead>Capacity</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {items.map((g) => (
                <TableRow
                  key={g.group_id}
                  className={`cursor-pointer ${
                    g.group_id === activeGroup ? "bg-muted" : ""
                  }`}
                  onClick={() => setSelected(g.group_id)}
                >
                  <TableCell className="font-mono text-xs">
                    {g.group_id.slice(0, 8)}
                  </TableCell>
                  <TableCell>{g.replica_count}</TableCell>
                  <TableCell>{formatBytes(g.size_bytes)}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
      <div className="lg:col-span-2">
        {activeGroup && (
          <GroupDetail backendId={backendId} groupId={activeGroup} />
        )}
      </div>
    </div>
  )
}

function GroupDetail({ backendId, groupId }: { backendId: string; groupId: string }) {
  const status = useRaftGroupStatus(backendId, groupId)
  const repair = useRepairReplica()

  if (status.isLoading) return <Loader />
  if (status.isError)
    return <ErrorBox label="group status">{(status.error as Error)?.message}</ErrorBox>
  const data = status.data!

  return (
    <Card>
      <CardHeader>
        <div className="flex items-start justify-between">
          <div>
            <CardTitle className="font-mono text-sm">{data.group_id}</CardTitle>
            <CardDescription>
              {formatBytes(data.size_bytes)} · block_size {data.block_size}
            </CardDescription>
          </div>
          <QuorumBadge state={data.quorum_state} />
        </div>
      </CardHeader>
      <CardContent>
        {data.lagging_followers.length > 0 && (
          <div className="mb-4 p-3 rounded-md border border-amber-300/40 bg-amber-50/30 dark:bg-amber-900/10 flex items-start gap-2 text-sm">
            <AlertCircle className="h-4 w-4 mt-0.5 text-amber-600 dark:text-amber-400" />
            <div>
              <div className="font-medium">Lagging followers</div>
              <div className="text-muted-foreground">
                Node id(s) {data.lagging_followers.join(", ")} are far behind the leader.
                Trigger repair to drive a catch-up.
              </div>
            </div>
          </div>
        )}
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Node</TableHead>
              <TableHead>Reachable</TableHead>
              <TableHead>Applied idx</TableHead>
              <TableHead>Store kind</TableHead>
              <TableHead>Action</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {data.replicas.map((r) => (
              <TableRow key={r.node_id}>
                <TableCell className="font-mono">{r.node_id}</TableCell>
                <TableCell>
                  {r.reachable ? (
                    <Badge variant="outline" className="text-emerald-700 border-emerald-700/40">
                      <CheckCircle2 className="h-3 w-3 mr-1" />
                      yes
                    </Badge>
                  ) : (
                    <Badge variant="destructive">
                      <AlertCircle className="h-3 w-3 mr-1" />
                      no
                    </Badge>
                  )}
                </TableCell>
                <TableCell className="font-mono">{r.last_applied_index ?? "—"}</TableCell>
                <TableCell className="font-mono text-xs">{r.store_kind ?? "—"}</TableCell>
                <TableCell>
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={repair.isPending}
                    onClick={() =>
                      repair.mutate({ backendId, groupId, nodeId: r.node_id })
                    }
                  >
                    <RefreshCw className="h-3 w-3 mr-1" />
                    Repair
                  </Button>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
        {repair.isError && (
          <div className="mt-3 text-sm text-destructive">
            {(repair.error as Error)?.message}
          </div>
        )}
      </CardContent>
    </Card>
  )
}

function HostsTab({ backendId: _backendId }: { backendId: string }) {
  const hosts = useHosts()
  const setHotSpare = useSetHostHotSpare()
  const decommission = useDecommissionHost()
  if (hosts.isLoading) return <Loader />
  if (hosts.isError) return <ErrorBox label="hosts">{(hosts.error as Error)?.message}</ErrorBox>

  const items = hosts.data ?? []
  return (
    <Card className="mt-4">
      <CardHeader>
        <CardTitle>Hosts</CardTitle>
        <CardDescription>
          Toggle hot-spare to reserve a host for failure recovery; decommission to begin a
          drain. Both operations are picked up by the auto-reconciler within ~60 s.
        </CardDescription>
      </CardHeader>
      <CardContent className="p-0">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Host</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Lifecycle</TableHead>
              <TableHead>Hot-spare</TableHead>
              <TableHead>SPDK backend</TableHead>
              <TableHead>Action</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {items.map((h) => (
              <TableRow key={h.id}>
                <TableCell>
                  <div className="font-medium">{h.name}</div>
                  <div className="text-xs text-muted-foreground font-mono">{h.addr}</div>
                </TableCell>
                <TableCell>{h.status}</TableCell>
                <TableCell>
                  <LifecycleBadge state={(h as { lifecycle_state?: string }).lifecycle_state ?? "active"} />
                </TableCell>
                <TableCell>
                  <Switch
                    checked={(h as { is_hot_spare?: boolean }).is_hot_spare ?? false}
                    disabled={setHotSpare.isPending}
                    onCheckedChange={(v) =>
                      setHotSpare.mutate({ hostId: h.id, isHotSpare: v })
                    }
                  />
                </TableCell>
                <TableCell className="font-mono text-xs">
                  {(h as { spdk_backend_id?: string | null }).spdk_backend_id?.slice(0, 8) ?? "—"}
                </TableCell>
                <TableCell>
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={
                      decommission.isPending ||
                      ((h as { lifecycle_state?: string }).lifecycle_state ?? "active") !== "active"
                    }
                    onClick={() => {
                      if (
                        window.confirm(
                          `Decommission host ${h.name}? The auto-reconciler will drain replicas onto a hot-spare and then mark the host as decommissioned. This cannot be reversed without re-registering the host.`
                        )
                      ) {
                        decommission.mutate({ hostId: h.id })
                      }
                    }}
                  >
                    Decommission
                  </Button>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
        {(setHotSpare.isError || decommission.isError) && (
          <div className="p-3 text-sm text-destructive">
            {(setHotSpare.error as Error)?.message ??
              (decommission.error as Error)?.message}
          </div>
        )}
      </CardContent>
    </Card>
  )
}

function RepairQueueTab({ backendId }: { backendId: string }) {
  const queue = useRaftRepairQueue(backendId)
  if (queue.isLoading) return <Loader />
  if (queue.isError)
    return <ErrorBox label="repair queue">{(queue.error as Error)?.message}</ErrorBox>
  const items = queue.data ?? []

  return (
    <Card className="mt-4">
      <CardHeader>
        <CardTitle>Repair queue</CardTitle>
        <CardDescription>
          Durable ledger of every membership operation. Stuck rows are auto-promoted to
          `failed` after 5 minutes; idempotent operations (repair) are auto-retried with
          exponential backoff.
        </CardDescription>
      </CardHeader>
      <CardContent className="p-0">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Op</TableHead>
              <TableHead>State</TableHead>
              <TableHead>Attempts</TableHead>
              <TableHead>Group</TableHead>
              <TableHead>Started</TableHead>
              <TableHead>Last error</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {items.length === 0 && (
              <TableRow>
                <TableCell colSpan={6} className="text-center text-muted-foreground py-6">
                  Queue is empty.
                </TableCell>
              </TableRow>
            )}
            {items.map((r) => (
              <TableRow key={r.id}>
                <TableCell className="font-mono text-xs">{r.op_type}</TableCell>
                <TableCell>
                  <QueueStateBadge state={r.state} />
                </TableCell>
                <TableCell>{r.attempts}</TableCell>
                <TableCell className="font-mono text-xs">
                  {r.group_id.slice(0, 8)}
                </TableCell>
                <TableCell className="text-xs">
                  {r.started_at ? new Date(r.started_at).toLocaleString() : "—"}
                </TableCell>
                <TableCell className="text-xs text-destructive max-w-md truncate">
                  {r.last_error ?? ""}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  )
}

function RebalanceTab({ backendId }: { backendId: string }) {
  const plan = useRebalancePlan(backendId)
  const execute = useExecutePlan()
  if (plan.isLoading) return <Loader />
  if (plan.isError)
    return <ErrorBox label="rebalance plan">{(plan.error as Error)?.message}</ErrorBox>
  const steps = plan.data?.plan.steps ?? []

  return (
    <Card className="mt-4">
      <CardHeader>
        <div className="flex items-start justify-between">
          <div>
            <CardTitle>Rebalance plan</CardTitle>
            <CardDescription>
              Read-only preview. Click Execute to apply the plan; each step holds a per-group
              advisory lock so quorum is preserved throughout.
            </CardDescription>
          </div>
          <Button
            disabled={execute.isPending || steps.length === 0}
            onClick={() => {
              if (
                window.confirm(
                  `Execute ${steps.length} rebalance step(s)? Replicas will move between hosts; this is reversible only by another rebalance.`
                )
              ) {
                execute.mutate({ backendId, plan: plan.data!.plan })
              }
            }}
          >
            {execute.isPending && <Loader2 className="h-3 w-3 mr-1 animate-spin" />}
            Execute
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {(plan.data?.plan.notes ?? []).map((note, i) => (
          <div key={i} className="text-sm text-muted-foreground mb-1">
            • {note}
          </div>
        ))}
        {steps.length === 0 ? (
          <div className="text-sm text-muted-foreground py-4">
            No moves needed — replication is already balanced.
          </div>
        ) : (
          <Table className="mt-3">
            <TableHeader>
              <TableRow>
                <TableHead>#</TableHead>
                <TableHead>Operation</TableHead>
                <TableHead>Group</TableHead>
                <TableHead>Detail</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {steps.map((s, i) => (
                <TableRow key={i}>
                  <TableCell>{i + 1}</TableCell>
                  <TableCell>{s.kind}</TableCell>
                  <TableCell className="font-mono text-xs">
                    {(s as { group_id: string }).group_id.slice(0, 8)}
                  </TableCell>
                  <TableCell className="font-mono text-xs">
                    {s.kind === "add_replica"
                      ? `→ node ${s.target_node_id} @ ${s.target_agent_base_url}`
                      : s.kind === "remove_replica"
                        ? `node ${s.node_id}`
                        : s.kind === "transfer_leader"
                          ? `${s.from_node_id} → ${s.to_node_id}`
                          : ""}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
        {execute.data && (
          <div className="mt-4 text-sm">
            Run completed in {execute.data.run.total_elapsed_ms} ms ·{" "}
            {execute.data.run.ok ? "all steps succeeded" : "stopped on first failure"}
          </div>
        )}
      </CardContent>
    </Card>
  )
}

// === Helpers ====================================================

function Loader() {
  return (
    <div className="flex items-center gap-2 p-6 text-muted-foreground">
      <Loader2 className="h-4 w-4 animate-spin" />
      Loading…
    </div>
  )
}

function ErrorBox({ children, label }: { children?: React.ReactNode; label: string }) {
  return (
    <div className="p-4 rounded-md border border-destructive/40 bg-destructive/5 flex items-start gap-2 text-sm">
      <ShieldAlert className="h-4 w-4 mt-0.5 text-destructive" />
      <div>
        <div className="font-medium text-destructive">Failed to load {label}</div>
        <div className="text-muted-foreground">{children}</div>
      </div>
    </div>
  )
}

function QuorumBadge({ state }: { state: string }) {
  if (state === "leader_steady")
    return <Badge className="bg-emerald-600/20 text-emerald-800 dark:text-emerald-300 border border-emerald-600/40">leader steady</Badge>
  if (state === "electing")
    return <Badge variant="outline" className="text-amber-700 border-amber-600/40">electing</Badge>
  return <Badge variant="destructive">quorum lost</Badge>
}

function LifecycleBadge({ state }: { state: string }) {
  if (state === "decommissioned") return <Badge variant="outline">decommissioned</Badge>
  if (state === "draining")
    return <Badge variant="outline" className="text-amber-700 border-amber-600/40">draining</Badge>
  return <Badge>active</Badge>
}

function QueueStateBadge({ state }: { state: string }) {
  if (state === "succeeded")
    return <Badge className="bg-emerald-600/20 text-emerald-800 dark:text-emerald-300 border border-emerald-600/40">succeeded</Badge>
  if (state === "failed") return <Badge variant="destructive">failed</Badge>
  if (state === "in_progress")
    return <Badge variant="outline" className="text-amber-700 border-amber-600/40">in progress</Badge>
  return <Badge variant="outline">{state}</Badge>
}

function formatBytes(n: number): string {
  if (n >= 1024 ** 4) return `${(n / 1024 ** 4).toFixed(1)} TiB`
  if (n >= 1024 ** 3) return `${(n / 1024 ** 3).toFixed(1)} GiB`
  if (n >= 1024 ** 2) return `${(n / 1024 ** 2).toFixed(1)} MiB`
  if (n >= 1024) return `${(n / 1024).toFixed(1)} KiB`
  return `${n} B`
}
