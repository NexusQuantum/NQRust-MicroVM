"use client"

import { useState, useEffect, useMemo } from "react"
import { Card, CardContent } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Slider } from "@/components/ui/slider"
import { Checkbox } from "@/components/ui/checkbox"
import { Textarea } from "@/components/ui/textarea"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { ChevronLeft, ChevronRight, Plus, Trash2, Disc, HardDrive, Monitor, ShieldCheck, Info } from "lucide-react"
import { useCreateVM, useNetworks, usePreferences, useHosts, useHostPciDevices } from "@/lib/queries"
import type { CreateVmReq, CreatePortForwardReq, GuestOs } from "@/lib/types"
import { BackendSelector } from "@/components/storage/backend-selector"

// Proxmox-style tabbed VM creation for the QEMU/UEFI tier. The user expresses
// intent (OS type + install source); the platform infers firmware (always
// UEFI), machine type (q35), virtio devices, TPM, and virtio-win driver
// attachment. None of that jargon is shown.
const TABS = [
  { id: "os", label: "OS" },
  { id: "system", label: "System" },
  { id: "disks", label: "Disks" },
  { id: "compute", label: "CPU & Memory" },
  { id: "network", label: "Network" },
  { id: "confirm", label: "Confirm" },
] as const

type TabId = (typeof TABS)[number]["id"]
type GuestKind = "linux" | "windows" | "other"
type Source = "iso" | "image"

interface QemuCreateWizardProps {
  onComplete?: () => void
  onCancel?: () => void
  onBack?: () => void
}

const GUEST_OS_API: Record<GuestKind, GuestOs> = {
  linux: "linux_disk",
  windows: "windows",
  other: "other",
}

export function QemuCreateWizard({ onComplete, onCancel, onBack }: QemuCreateWizardProps) {
  const createVM = useCreateVM()
  const { data: networks } = useNetworks()
  const { data: preferences } = usePreferences()
  const { data: hosts } = useHosts()
  const pciHostId = hosts?.[0]?.id
  const { data: pciDevices = [] } = useHostPciDevices(pciHostId)

  const [tab, setTab] = useState<TabId>("os")

  // OS tab
  const [name, setName] = useState("")
  const [guestKind, setGuestKind] = useState<GuestKind>("linux")
  const [source, setSource] = useState<Source>("iso")
  const [installerIsoId, setInstallerIsoId] = useState<string | undefined>(undefined)
  const [diskImageId, setDiskImageId] = useState<string | undefined>(undefined)
  // Cloud-init credentials (cloud-image source only)
  const [username, setUsername] = useState("root")
  const [password, setPassword] = useState("")
  const [sshKeys, setSshKeys] = useState("")

  // System tab
  const [enableVnc, setEnableVnc] = useState(true)

  // Disks tab
  const [diskSizeGb, setDiskSizeGb] = useState<number>(40)
  const [backendId, setBackendId] = useState<string | undefined>(undefined)
  // Additional blank data disks (sizes in GB).
  const [extraDisks, setExtraDisks] = useState<number[]>([])

  // System tab — PCI passthrough (advanced): selected host PCI BDFs.
  const [vfioDevices, setVfioDevices] = useState<string[]>([])

  // Compute tab
  const [vcpu, setVcpu] = useState<number>(2)
  const [memMib, setMemMib] = useState<number>(4096)
  const [cpuType, setCpuType] = useState<string>("host")

  // Network tab
  const [networkId, setNetworkId] = useState<string | undefined>(undefined)
  // Additional networks (each becomes its own NIC).
  const [extraNetworkIds, setExtraNetworkIds] = useState<string[]>([])
  const [portForwards, setPortForwards] = useState<CreatePortForwardReq[]>([])
  const [pfHostPort, setPfHostPort] = useState("")
  const [pfGuestPort, setPfGuestPort] = useState("")
  const [pfProtocol, setPfProtocol] = useState("tcp")
  const [pfDescription, setPfDescription] = useState("")

  // Image registry options
  const [isoOptions, setIsoOptions] = useState<{ id: string; name: string }[]>([])
  const [diskImageOptions, setDiskImageOptions] = useState<{ id: string; name: string; image_kind: string }[]>([])
  const [hasVirtioWin, setHasVirtioWin] = useState(false)

  // Apply preference defaults
  useEffect(() => {
    if (preferences?.vm_defaults) {
      setVcpu(preferences.vm_defaults.vcpu || 2)
      setMemMib(preferences.vm_defaults.mem_mib || 4096)
    }
  }, [preferences])

  // Default network
  useEffect(() => {
    if (networks && networks.length > 0 && !networkId) {
      const def = networks.find((n) => n.name === "Default Network") || networks[0]
      setNetworkId(def.id)
    }
  }, [networks, networkId])

  // VNC sensible default: graphical installers (Windows/Other, or Linux from
  // ISO) want a console; headless Linux cloud images don't need one.
  useEffect(() => {
    setEnableVnc(guestKind !== "linux" || source === "iso")
  }, [guestKind, source])

  // Load images from registry
  useEffect(() => {
    ;(async () => {
      try {
        const baseUrl =
          process.env.NEXT_PUBLIC_API_BASE_URL ||
          `${window.location.protocol}//${window.location.hostname}:18080/v1`
        const res = await fetch(`${baseUrl}/images`)
        const data = await res.json()
        const items: any[] = data.items || []

        setIsoOptions(
          items
            .filter((i) => i.image_kind === "installer_iso")
            .map((i) => ({ id: i.id, name: i.name })),
        )
        setDiskImageOptions(
          items
            .filter((i) => i.image_kind === "uefi_disk" || i.image_kind === "linux_disk")
            .map((i) => ({ id: i.id, name: i.name, image_kind: i.image_kind })),
        )
        setHasVirtioWin(
          items.some(
            (i) =>
              i.image_kind === "installer_iso" &&
              (i.name || "").toLowerCase().includes("virtio-win"),
          ),
        )
      } catch (e) {
        console.error("Failed to load images:", e)
      }
    })()
  }, [])

  const isWindows = guestKind === "windows"

  // ---- Per-tab validity (gates the final Create button + tab hints) ----
  const osValid =
    name.trim().length > 0 &&
    ((source === "iso" && !!installerIsoId) || (source === "image" && !!diskImageId))
  const disksValid = source === "image" ? true : diskSizeGb >= 1
  const computeValid = vcpu >= 1 && memMib >= 512
  const canCreate = osValid && disksValid && computeValid

  const tabIndex = TABS.findIndex((t) => t.id === tab)
  const goNext = () => {
    if (tabIndex < TABS.length - 1) setTab(TABS[tabIndex + 1].id)
  }
  const goPrev = () => {
    if (tabIndex > 0) setTab(TABS[tabIndex - 1].id)
    else onBack?.()
  }

  const summaryLine = useMemo(() => {
    const os = isWindows ? "Windows" : guestKind === "linux" ? "Linux" : "Custom OS"
    const src =
      source === "iso"
        ? `install from ${isoOptions.find((i) => i.id === installerIsoId)?.name || "ISO"}`
        : `boot ${diskImageOptions.find((i) => i.id === diskImageId)?.name || "disk image"}`
    const diskTxt = source === "image" ? "image disk" : `${diskSizeGb} GB disk`
    return `${os} VM — ${vcpu} vCPU / ${(memMib / 1024).toFixed(memMib % 1024 ? 1 : 0)} GB, ${diskTxt}, ${src}`
  }, [isWindows, guestKind, source, installerIsoId, diskImageId, isoOptions, diskImageOptions, diskSizeGb, vcpu, memMib])

  const handleCreate = async () => {
    if (!canCreate) return
    const sshList = sshKeys
      .split("\n")
      .map((k) => k.trim())
      .filter(Boolean)

    const req: CreateVmReq = {
      name: name.trim(),
      vcpu,
      mem_mib: memMib,
      cpu_type: cpuType,
      vmm_kind: "qemu",
      guest_os: GUEST_OS_API[guestKind],
      enable_vnc: enableVnc,
      disk_image_id: source === "image" ? diskImageId : undefined,
      installer_iso_id: source === "iso" ? installerIsoId : undefined,
      rootfs_size_mb: source === "iso" || diskSizeGb ? diskSizeGb * 1024 : undefined,
      network_id: networkId || undefined,
      extra_network_ids: extraNetworkIds.filter(Boolean),
      port_forwards: portForwards.length > 0 ? portForwards : undefined,
      data_disks: extraDisks.filter((g) => g > 0).map((g) => ({ size_mb: g * 1024 })),
      vfio_devices: vfioDevices,
      ...(backendId ? { backend_id: backendId } : {}),
      // Cloud-init / cloudbase-init credentials only make sense for cloud images.
      ...(source === "image"
        ? {
            username: username || undefined,
            password: password || undefined,
            ssh_authorized_keys: sshList.length > 0 ? sshList : undefined,
          }
        : {}),
      tags: [],
    }
    try {
      await createVM.mutateAsync(req)
      onComplete?.()
    } catch (e) {
      console.error("VM creation failed:", e)
    }
  }

  return (
    <div className="space-y-6">
      <Tabs value={tab} onValueChange={(v) => setTab(v as TabId)}>
        <TabsList className="grid w-full grid-cols-6">
          {TABS.map((t) => (
            <TabsTrigger key={t.id} value={t.id}>
              {t.label}
            </TabsTrigger>
          ))}
        </TabsList>

        <Card className="mt-4">
          <CardContent className="space-y-5 pt-6">
            {/* ---------------- OS ---------------- */}
            <TabsContent value="os" className="mt-0 space-y-5">
              <div className="space-y-2">
                <Label htmlFor="qemu-name">
                  Name <span className="text-destructive">*</span>
                </Label>
                <Input
                  id="qemu-name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="e.g., win11-desktop or ubuntu-server"
                />
              </div>

              <div className="space-y-2">
                <Label>Operating system</Label>
                <div className="grid grid-cols-3 gap-3">
                  {(
                    [
                      { k: "linux", label: "Linux", hint: "Ubuntu, Debian, Fedora…" },
                      { k: "windows", label: "Windows", hint: "Win 10/11, Server" },
                      { k: "other", label: "Other", hint: "BSD, custom ISO" },
                    ] as { k: GuestKind; label: string; hint: string }[]
                  ).map((o) => (
                    <button
                      key={o.k}
                      type="button"
                      onClick={() => setGuestKind(o.k)}
                      className={`rounded-lg border-2 p-3 text-left transition-colors ${
                        guestKind === o.k ? "border-primary bg-primary/5" : "border-muted hover:border-primary/50"
                      }`}
                    >
                      <div className="font-semibold">{o.label}</div>
                      <div className="mt-1 text-xs text-muted-foreground">{o.hint}</div>
                    </button>
                  ))}
                </div>
              </div>

              <div className="space-y-2">
                <Label>Installation source</Label>
                <div className="grid grid-cols-2 gap-3">
                  <button
                    type="button"
                    onClick={() => setSource("iso")}
                    className={`flex items-start gap-3 rounded-lg border-2 p-3 text-left transition-colors ${
                      source === "iso" ? "border-primary bg-primary/5" : "border-muted hover:border-primary/50"
                    }`}
                  >
                    <Disc className="mt-0.5 h-5 w-5 shrink-0 text-muted-foreground" />
                    <div>
                      <div className="font-semibold">Install from ISO</div>
                      <div className="mt-1 text-xs text-muted-foreground">
                        Fresh install onto a new blank disk.
                      </div>
                    </div>
                  </button>
                  <button
                    type="button"
                    onClick={() => setSource("image")}
                    className={`flex items-start gap-3 rounded-lg border-2 p-3 text-left transition-colors ${
                      source === "image" ? "border-primary bg-primary/5" : "border-muted hover:border-primary/50"
                    }`}
                  >
                    <HardDrive className="mt-0.5 h-5 w-5 shrink-0 text-muted-foreground" />
                    <div>
                      <div className="font-semibold">Boot a disk image</div>
                      <div className="mt-1 text-xs text-muted-foreground">
                        Ready-to-run cloud / disk image (no install).
                      </div>
                    </div>
                  </button>
                </div>
              </div>

              {source === "iso" ? (
                <div className="space-y-2">
                  <Label htmlFor="iso">
                    ISO image <span className="text-destructive">*</span>
                  </Label>
                  <Select value={installerIsoId} onValueChange={setInstallerIsoId}>
                    <SelectTrigger id="iso">
                      <SelectValue
                        placeholder={isoOptions.length ? "Select an ISO" : "No ISOs — upload one on the Registry page"}
                      />
                    </SelectTrigger>
                    <SelectContent>
                      {isoOptions.map((iso) => (
                        <SelectItem key={iso.id} value={iso.id}>
                          {iso.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  {!isoOptions.length && (
                    <p className="text-xs text-muted-foreground">
                      Upload an installer ISO on the Registry page (kind: installer ISO).
                    </p>
                  )}
                </div>
              ) : (
                <div className="space-y-2">
                  <Label htmlFor="disk-image">
                    Disk image <span className="text-destructive">*</span>
                  </Label>
                  <Select value={diskImageId} onValueChange={setDiskImageId}>
                    <SelectTrigger id="disk-image">
                      <SelectValue
                        placeholder={
                          diskImageOptions.length ? "Select a disk image" : "No disk images — upload one on the Registry page"
                        }
                      />
                    </SelectTrigger>
                    <SelectContent>
                      {diskImageOptions.map((img) => (
                        <SelectItem key={img.id} value={img.id}>
                          {img.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <p className="text-xs text-muted-foreground">
                    A per-VM copy-on-write overlay is created so the base image stays untouched.
                  </p>
                </div>
              )}

              {isWindows && source === "iso" && (
                <div
                  className={`flex items-start gap-2 rounded-md border p-3 text-xs ${
                    hasVirtioWin ? "border-muted bg-muted/40 text-muted-foreground" : "border-amber-500/40 bg-amber-500/10"
                  }`}
                >
                  <Info className="mt-0.5 h-4 w-4 shrink-0" />
                  {hasVirtioWin ? (
                    <span>VirtIO drivers will be auto-attached as a second CD-ROM so Windows Setup sees the disk and network.</span>
                  ) : (
                    <span>
                      No <strong>virtio-win</strong> ISO found in the registry. Windows Setup won&apos;t see the disk/network
                      until you upload one (Registry → installer ISO, name containing &quot;virtio-win&quot;).
                    </span>
                  )}
                </div>
              )}

              {/* Cloud-init credentials — only meaningful for cloud images */}
              {source === "image" && (
                <div className="space-y-4 rounded-lg border border-border p-4">
                  <div className="text-sm font-medium">First-boot credentials (cloud-init)</div>
                  <div className="grid grid-cols-2 gap-3">
                    <div className="space-y-2">
                      <Label htmlFor="cu">Username</Label>
                      <Input id="cu" value={username} onChange={(e) => setUsername(e.target.value)} />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="cp">Password</Label>
                      <Input id="cp" type="password" value={password} onChange={(e) => setPassword(e.target.value)} />
                    </div>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="ck">SSH authorized keys (optional)</Label>
                    <Textarea
                      id="ck"
                      rows={2}
                      value={sshKeys}
                      onChange={(e) => setSshKeys(e.target.value)}
                      placeholder="ssh-ed25519 AAAA... user@host"
                    />
                  </div>
                  <p className="text-xs text-muted-foreground">
                    Injected on first boot via cloud-init (Linux) or cloudbase-init (Windows).
                  </p>
                </div>
              )}
            </TabsContent>

            {/* ---------------- System ---------------- */}
            <TabsContent value="system" className="mt-0 space-y-5">
              <div className="flex items-start gap-2 rounded-md border border-muted bg-muted/40 p-3 text-xs text-muted-foreground">
                <ShieldCheck className="mt-0.5 h-4 w-4 shrink-0" />
                <span>
                  This VM boots with <strong>UEFI</strong> firmware on a modern machine profile with paravirtual
                  (VirtIO) devices — configured automatically.
                  {isWindows && " TPM 2.0 is enabled automatically for Windows."}
                </span>
              </div>

              <label className="flex items-start gap-3 rounded-lg border border-border p-4 cursor-pointer">
                <Checkbox checked={enableVnc} onCheckedChange={(c) => setEnableVnc(c as boolean)} className="mt-0.5" />
                <div>
                  <div className="flex items-center gap-2 font-medium">
                    <Monitor className="h-4 w-4" /> Graphical console (VNC)
                  </div>
                  <p className="mt-1 text-xs text-muted-foreground">
                    Required for graphical installers (Windows, desktop Linux). Headless servers can use the serial
                    console instead.
                  </p>
                </div>
              </label>

              {isWindows && (
                <div className="flex items-start gap-2 rounded-md border border-muted bg-muted/40 p-3 text-xs text-muted-foreground">
                  <ShieldCheck className="mt-0.5 h-4 w-4 shrink-0" />
                  <span>TPM 2.0 device added automatically (required by Windows 11).</span>
                </div>
              )}

              <div className="space-y-2 border-t pt-4">
                <Label>PCI passthrough (advanced)</Label>
                <p className="text-xs text-muted-foreground">
                  Pass host PCI devices (GPUs, NICs) straight through to the guest. Requires IOMMU + the
                  device bound to <code>vfio-pci</code> on the host.
                </p>
                <div className="max-h-56 overflow-y-auto rounded-md border divide-y">
                  {pciDevices.length === 0 ? (
                    <div className="px-3 py-2 text-sm text-muted-foreground">
                      No PCI devices reported by the host.
                    </div>
                  ) : (
                    pciDevices.map((d) => {
                      const checked = vfioDevices.includes(d.bdf)
                      return (
                        <label
                          key={d.bdf}
                          className="flex cursor-pointer items-center gap-3 px-3 py-2 hover:bg-muted/50"
                        >
                          <Checkbox
                            checked={checked}
                            onCheckedChange={(c) =>
                              setVfioDevices((prev) =>
                                c ? [...prev, d.bdf] : prev.filter((b) => b !== d.bdf),
                              )
                            }
                          />
                          <div className="min-w-0 flex-1">
                            <div className="truncate text-sm">{d.label || d.bdf}</div>
                            <div className="truncate text-xs text-muted-foreground">
                              <span className="font-mono">{d.bdf}</span>
                              {d.class_name ? ` · ${d.class_name}` : ""}
                              {d.driver ? ` · driver: ${d.driver}` : ""}
                            </div>
                          </div>
                        </label>
                      )
                    })
                  )}
                </div>
                {vfioDevices.length > 0 && (
                  <p className="text-xs text-muted-foreground">{vfioDevices.length} device(s) selected.</p>
                )}
              </div>
            </TabsContent>

            {/* ---------------- Disks ---------------- */}
            <TabsContent value="disks" className="mt-0 space-y-5">
              {source === "image" ? (
                <p className="text-sm text-muted-foreground">
                  Booting an existing disk image — the disk comes from the image. Set a size below only if you want to
                  grow it.
                </p>
              ) : (
                <p className="text-sm text-muted-foreground">A new blank disk will be created to install onto.</p>
              )}

              <div className="space-y-2">
                <Label>
                  Disk size: {diskSizeGb} GB
                  {source === "image" && <span className="ml-1 text-muted-foreground">(optional — grow only)</span>}
                </Label>
                <Slider value={[diskSizeGb]} onValueChange={(v) => setDiskSizeGb(v[0])} min={1} max={1024} step={1} />
                <div className="flex items-center gap-2">
                  <Input
                    type="number"
                    className="w-28"
                    value={diskSizeGb}
                    min={1}
                    max={1024}
                    onChange={(e) => setDiskSizeGb(Math.max(1, parseInt(e.target.value) || 1))}
                  />
                  <span className="text-sm text-muted-foreground">GB</span>
                </div>
              </div>

              <BackendSelector id="qemu-backend" value={backendId} onChange={setBackendId} />
              <p className="text-xs text-muted-foreground">
                Storage location for the VM disk. Defaults to local storage when only one is available.
              </p>

              <div className="space-y-3 border-t pt-4">
                <div className="flex items-center justify-between">
                  <Label>Additional disks</Label>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => setExtraDisks((d) => [...d, 20])}
                  >
                    <Plus className="mr-1 h-4 w-4" /> Add disk
                  </Button>
                </div>
                {extraDisks.length === 0 && (
                  <p className="text-xs text-muted-foreground">
                    Extra blank data disks attached alongside the boot disk (e.g. for data/storage).
                  </p>
                )}
                {extraDisks.map((sz, i) => (
                  <div key={i} className="flex items-center gap-2">
                    <span className="w-16 text-sm text-muted-foreground">Disk {i + 2}</span>
                    <Input
                      type="number"
                      className="w-28"
                      min={1}
                      max={4096}
                      value={sz}
                      onChange={(e) =>
                        setExtraDisks((d) =>
                          d.map((v, idx) => (idx === i ? Math.max(1, parseInt(e.target.value) || 1) : v)),
                        )
                      }
                    />
                    <span className="text-sm text-muted-foreground">GB</span>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7"
                      onClick={() => setExtraDisks((d) => d.filter((_, idx) => idx !== i))}
                    >
                      <Trash2 className="h-3.5 w-3.5 text-destructive" />
                    </Button>
                  </div>
                ))}
              </div>
            </TabsContent>

            {/* ---------------- CPU & Memory ---------------- */}
            <TabsContent value="compute" className="mt-0 space-y-5">
              <div className="space-y-2">
                <Label htmlFor="cpu-type">CPU type</Label>
                <Select value={cpuType} onValueChange={setCpuType}>
                  <SelectTrigger id="cpu-type">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="host">host (all host features — best performance, nested virt)</SelectItem>
                    <SelectItem value="max">max (everything QEMU + host support)</SelectItem>
                    <SelectItem value="kvm64">kvm64 (safe baseline — best migration compatibility)</SelectItem>
                    <SelectItem value="qemu64">qemu64</SelectItem>
                    <SelectItem value="x86-64-v2-AES">x86-64-v2-AES</SelectItem>
                    <SelectItem value="x86-64-v3">x86-64-v3 (modern baseline)</SelectItem>
                    <SelectItem value="x86-64-v4">x86-64-v4 (AVX-512)</SelectItem>
                    <SelectItem value="Skylake-Server">Intel Skylake-Server</SelectItem>
                    <SelectItem value="Cascadelake-Server">Intel Cascadelake-Server</SelectItem>
                    <SelectItem value="EPYC">AMD EPYC</SelectItem>
                    <SelectItem value="EPYC-Rome">AMD EPYC-Rome</SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  <strong>host</strong> gives the guest all CPU features (needed for nested VMs) but can&apos;t
                  live-migrate to a different CPU. Pick a fixed model for migration compatibility.
                </p>
              </div>
              <div className="space-y-2">
                <Label>vCPU: {vcpu}</Label>
                <Slider value={[vcpu]} onValueChange={(v) => setVcpu(v[0])} min={1} max={32} step={1} />
              </div>
              <div className="space-y-2">
                <Label>Memory: {(memMib / 1024).toFixed(memMib % 1024 ? 1 : 0)} GB ({memMib} MiB)</Label>
                <Slider value={[memMib]} onValueChange={(v) => setMemMib(v[0])} min={512} max={65536} step={512} />
              </div>
            </TabsContent>

            {/* ---------------- Network ---------------- */}
            <TabsContent value="network" className="mt-0 space-y-5">
              <div className="space-y-2">
                <Label htmlFor="net">Network</Label>
                <Select value={networkId || ""} onValueChange={setNetworkId}>
                  <SelectTrigger id="net">
                    <SelectValue placeholder={networks?.length ? "Select network" : "No networks available"} />
                  </SelectTrigger>
                  <SelectContent>
                    {(networks || []).map((n) => (
                      <SelectItem key={n.id} value={n.id}>
                        {n.name} ({n.bridge_name} - {n.type})
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <Label>Additional networks</Label>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    disabled={!networks?.length}
                    onClick={() => setExtraNetworkIds((n) => [...n, networks?.[0]?.id || ""])}
                  >
                    <Plus className="mr-1 h-4 w-4" /> Add NIC
                  </Button>
                </div>
                {extraNetworkIds.length === 0 && (
                  <p className="text-xs text-muted-foreground">
                    Attach the VM to more than one network — each adds a separate virtio NIC.
                  </p>
                )}
                {extraNetworkIds.map((nid, i) => (
                  <div key={i} className="flex items-center gap-2">
                    <span className="w-14 text-sm text-muted-foreground">NIC {i + 2}</span>
                    <Select
                      value={nid || ""}
                      onValueChange={(v) =>
                        setExtraNetworkIds((arr) => arr.map((x, idx) => (idx === i ? v : x)))
                      }
                    >
                      <SelectTrigger className="flex-1">
                        <SelectValue placeholder="Select network" />
                      </SelectTrigger>
                      <SelectContent>
                        {(networks || []).map((n) => (
                          <SelectItem key={n.id} value={n.id}>
                            {n.name} ({n.bridge_name} - {n.type})
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7"
                      onClick={() => setExtraNetworkIds((arr) => arr.filter((_, idx) => idx !== i))}
                    >
                      <Trash2 className="h-3.5 w-3.5 text-destructive" />
                    </Button>
                  </div>
                ))}
              </div>

              <div className="space-y-3 border-t pt-4">
                <Label>Port forwarding (optional)</Label>
                {portForwards.length > 0 && (
                  <div className="rounded-md border">
                    <table className="w-full text-sm">
                      <tbody>
                        {portForwards.map((pf, i) => (
                          <tr key={i} className="border-b last:border-0">
                            <td className="px-3 py-2 font-mono text-xs uppercase">{pf.protocol || "tcp"}</td>
                            <td className="px-3 py-2 font-mono">
                              {pf.host_port} → {pf.guest_port}
                            </td>
                            <td className="px-3 py-2 text-muted-foreground">{pf.description || "—"}</td>
                            <td className="px-3 py-2">
                              <Button
                                type="button"
                                variant="ghost"
                                size="icon"
                                className="h-7 w-7"
                                onClick={() => setPortForwards((prev) => prev.filter((_, idx) => idx !== i))}
                              >
                                <Trash2 className="h-3.5 w-3.5 text-destructive" />
                              </Button>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                )}
                <div className="flex items-end gap-2">
                  <div className="space-y-1">
                    <Label className="text-xs">Host</Label>
                    <Input
                      type="number"
                      className="w-24"
                      placeholder="8080"
                      value={pfHostPort}
                      onChange={(e) => setPfHostPort(e.target.value)}
                    />
                  </div>
                  <div className="space-y-1">
                    <Label className="text-xs">Guest</Label>
                    <Input
                      type="number"
                      className="w-24"
                      placeholder="80"
                      value={pfGuestPort}
                      onChange={(e) => setPfGuestPort(e.target.value)}
                    />
                  </div>
                  <div className="space-y-1">
                    <Label className="text-xs">Protocol</Label>
                    <Select value={pfProtocol} onValueChange={setPfProtocol}>
                      <SelectTrigger className="w-24">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="tcp">TCP</SelectItem>
                        <SelectItem value="udp">UDP</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="flex-1 space-y-1">
                    <Label className="text-xs">Description</Label>
                    <Input
                      placeholder="e.g., RDP"
                      value={pfDescription}
                      onChange={(e) => setPfDescription(e.target.value)}
                    />
                  </div>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    disabled={!pfHostPort || !pfGuestPort}
                    onClick={() => {
                      const hp = parseInt(pfHostPort)
                      const gp = parseInt(pfGuestPort)
                      if (hp >= 1 && hp <= 65535 && gp >= 1 && gp <= 65535) {
                        setPortForwards((prev) => [
                          ...prev,
                          { host_port: hp, guest_port: gp, protocol: pfProtocol, description: pfDescription || undefined },
                        ])
                        setPfHostPort("")
                        setPfGuestPort("")
                        setPfDescription("")
                      }
                    }}
                  >
                    <Plus className="mr-1 h-4 w-4" /> Add
                  </Button>
                </div>
              </div>
            </TabsContent>

            {/* ---------------- Confirm ---------------- */}
            <TabsContent value="confirm" className="mt-0 space-y-4">
              <div className="rounded-lg border-l-4 border-l-primary bg-primary/5 p-4">
                <p className="text-sm font-medium">{summaryLine}</p>
                <p className="mt-1 text-xs text-muted-foreground">Review before creating. You can revisit any tab.</p>
              </div>
              <div className="rounded-lg border border-border p-4">
                <dl className="grid grid-cols-2 gap-2 text-sm">
                  <dt className="text-muted-foreground">Name</dt>
                  <dd>{name || "—"}</dd>
                  <dt className="text-muted-foreground">Operating system</dt>
                  <dd className="capitalize">{guestKind}</dd>
                  <dt className="text-muted-foreground">Source</dt>
                  <dd>
                    {source === "iso"
                      ? `Install from ${isoOptions.find((i) => i.id === installerIsoId)?.name || "— (select an ISO)"}`
                      : `Boot ${diskImageOptions.find((i) => i.id === diskImageId)?.name || "— (select an image)"}`}
                  </dd>
                  <dt className="text-muted-foreground">vCPU / Memory</dt>
                  <dd>
                    {vcpu} / {memMib} MiB
                  </dd>
                  <dt className="text-muted-foreground">Disk</dt>
                  <dd>{source === "image" ? "From image" : `${diskSizeGb} GB (new)`}</dd>
                  <dt className="text-muted-foreground">Console</dt>
                  <dd>{enableVnc ? "Graphical (VNC)" : "Serial"}</dd>
                  <dt className="text-muted-foreground">Network</dt>
                  <dd>{networks?.find((n) => n.id === networkId)?.name || "Default"}</dd>
                </dl>
              </div>
              {!canCreate && (
                <p className="text-sm text-amber-600">
                  Complete the required fields on the OS tab{!disksValid ? " and set a disk size" : ""} before creating.
                </p>
              )}
            </TabsContent>
          </CardContent>
        </Card>
      </Tabs>

      <div className="flex items-center justify-between">
        <div className="flex gap-2">
          {onCancel && (
            <Button type="button" variant="outline" onClick={onCancel}>
              Cancel
            </Button>
          )}
          <Button type="button" variant="outline" onClick={goPrev}>
            <ChevronLeft className="mr-2 h-4 w-4" />
            {tabIndex === 0 ? "Back" : "Previous"}
          </Button>
        </div>
        {tab !== "confirm" ? (
          <Button type="button" onClick={goNext}>
            Next
            <ChevronRight className="ml-2 h-4 w-4" />
          </Button>
        ) : (
          <Button type="button" onClick={handleCreate} disabled={!canCreate || createVM.isPending}>
            {createVM.isPending ? "Creating VM…" : "Create VM"}
          </Button>
        )}
      </div>
    </div>
  )
}
