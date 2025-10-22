"use client"

import { useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Slider } from "@/components/ui/slider"
import { Checkbox } from "@/components/ui/checkbox"
import { Textarea } from "@/components/ui/textarea"
import { ChevronLeft, ChevronRight } from "lucide-react"

const steps = ["Basic Info", "Credentials", "Machine Config", "Boot Source", "Network", "Review"]

export function VMCreateWizard() {
  const [currentStep, setCurrentStep] = useState(0)

  // Form state
  const [name, setName] = useState("")
  const [description, setDescription] = useState("")
  const [username, setUsername] = useState("root")
  const [password, setPassword] = useState("")
  const [vcpu, setVcpu] = useState(2)
  const [memory, setMemory] = useState(2048)
  const [smtEnabled, setSmtEnabled] = useState(false)
  const [trackDirtyPages, setTrackDirtyPages] = useState(false)
  const [kernelPath, setKernelPath] = useState("")
  const [rootfsPath, setRootfsPath] = useState("")
  const [initrdPath, setInitrdPath] = useState("")
  const [bootArgs, setBootArgs] = useState("")
  const [enableNetwork, setEnableNetwork] = useState(true)
  const [hostDevice, setHostDevice] = useState("tap0")
  const [guestMac, setGuestMac] = useState("")

  const nextStep = () => {
    if (currentStep < steps.length - 1) {
      setCurrentStep(currentStep + 1)
    }
  }

  const prevStep = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1)
    }
  }

  const generateMac = () => {
    const mac = Array.from({ length: 6 }, () =>
      Math.floor(Math.random() * 256)
        .toString(16)
        .padStart(2, "0"),
    ).join(":")
    setGuestMac(mac)
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        {steps.map((step, index) => (
          <div key={step} className="flex items-center">
            <div
              className={`flex h-10 w-10 items-center justify-center rounded-full border-2 ${
                index === currentStep
                  ? "border-primary bg-primary text-primary-foreground"
                  : index < currentStep
                    ? "border-primary bg-primary text-primary-foreground"
                    : "border-muted bg-muted text-muted-foreground"
              }`}
            >
              {index + 1}
            </div>
            <div className="ml-2 text-sm font-medium">{step}</div>
            {index < steps.length - 1 && <div className="mx-4 h-0.5 w-12 bg-muted" />}
          </div>
        ))}
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{steps[currentStep]}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {currentStep === 0 && (
            <>
              <div className="space-y-2">
                <Label htmlFor="name">
                  Name <span className="text-destructive">*</span>
                </Label>
                <Input id="name" value={name} onChange={(e) => setName(e.target.value)} placeholder="my-vm" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="description">Description</Label>
                <Textarea
                  id="description"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder="Optional description"
                />
              </div>
            </>
          )}

          {currentStep === 1 && (
            <>
              <div className="space-y-2">
                <Label htmlFor="username">Username</Label>
                <Input id="username" value={username} onChange={(e) => setUsername(e.target.value)} />
              </div>
              <div className="space-y-2">
                <Label htmlFor="password">
                  Password <span className="text-destructive">*</span>
                </Label>
                <Input
                  id="password"
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder="Enter password"
                />
              </div>
            </>
          )}

          {currentStep === 2 && (
            <>
              <div className="space-y-2">
                <Label>vCPU Count: {vcpu}</Label>
                <Slider value={[vcpu]} onValueChange={(v) => setVcpu(v[0])} min={1} max={32} step={1} />
              </div>
              <div className="space-y-2">
                <Label>Memory: {memory} MiB</Label>
                <Slider value={[memory]} onValueChange={(v) => setMemory(v[0])} min={128} max={32768} step={128} />
              </div>
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="smt"
                  checked={smtEnabled}
                  onCheckedChange={(checked) => setSmtEnabled(checked as boolean)}
                />
                <Label htmlFor="smt" className="text-sm font-normal">
                  Enable SMT (Simultaneous Multithreading)
                </Label>
              </div>
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="dirty-pages"
                  checked={trackDirtyPages}
                  onCheckedChange={(checked) => setTrackDirtyPages(checked as boolean)}
                />
                <Label htmlFor="dirty-pages" className="text-sm font-normal">
                  Track dirty pages
                </Label>
              </div>
            </>
          )}

          {currentStep === 3 && (
            <>
              <div className="space-y-2">
                <Label htmlFor="kernel">
                  Kernel Image <span className="text-destructive">*</span>
                </Label>
                <Select value={kernelPath} onValueChange={setKernelPath}>
                  <SelectTrigger id="kernel">
                    <SelectValue placeholder="Select kernel image" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="/images/vmlinux-5.10">vmlinux-5.10</SelectItem>
                    <SelectItem value="/images/vmlinux-5.15">vmlinux-5.15</SelectItem>
                    <SelectItem value="/images/vmlinux-6.1">vmlinux-6.1</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="rootfs">
                  Rootfs Image <span className="text-destructive">*</span>
                </Label>
                <Select value={rootfsPath} onValueChange={setRootfsPath}>
                  <SelectTrigger id="rootfs">
                    <SelectValue placeholder="Select rootfs image" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="/images/ubuntu-22.04.ext4">ubuntu-22.04-rootfs</SelectItem>
                    <SelectItem value="/images/alpine.ext4">alpine-rootfs</SelectItem>
                    <SelectItem value="/images/debian-12.ext4">debian-12-rootfs</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="initrd">Initrd Path (Optional)</Label>
                <Input id="initrd" value={initrdPath} onChange={(e) => setInitrdPath(e.target.value)} />
              </div>
              <div className="space-y-2">
                <Label htmlFor="boot-args">Boot Arguments (Optional)</Label>
                <Input id="boot-args" value={bootArgs} onChange={(e) => setBootArgs(e.target.value)} />
              </div>
            </>
          )}

          {currentStep === 4 && (
            <>
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="enable-network"
                  checked={enableNetwork}
                  onCheckedChange={(checked) => setEnableNetwork(checked as boolean)}
                />
                <Label htmlFor="enable-network" className="text-sm font-normal">
                  Enable networking
                </Label>
              </div>
              {enableNetwork && (
                <>
                  <div className="space-y-2">
                    <Label htmlFor="host-device">Host Device Name</Label>
                    <Input id="host-device" value={hostDevice} onChange={(e) => setHostDevice(e.target.value)} />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="guest-mac">Guest MAC Address</Label>
                    <div className="flex gap-2">
                      <Input id="guest-mac" value={guestMac} onChange={(e) => setGuestMac(e.target.value)} />
                      <Button type="button" variant="outline" onClick={generateMac}>
                        Generate
                      </Button>
                    </div>
                  </div>
                </>
              )}
            </>
          )}

          {currentStep === 5 && (
            <div className="space-y-4">
              <div className="rounded-lg border border-border p-4 space-y-3">
                <h3 className="font-medium">Basic Information</h3>
                <dl className="grid grid-cols-2 gap-2 text-sm">
                  <dt className="text-muted-foreground">Name:</dt>
                  <dd>{name || "—"}</dd>
                  <dt className="text-muted-foreground">Description:</dt>
                  <dd>{description || "—"}</dd>
                </dl>
              </div>

              <div className="rounded-lg border border-border p-4 space-y-3">
                <h3 className="font-medium">Machine Configuration</h3>
                <dl className="grid grid-cols-2 gap-2 text-sm">
                  <dt className="text-muted-foreground">vCPU:</dt>
                  <dd>{vcpu}</dd>
                  <dt className="text-muted-foreground">Memory:</dt>
                  <dd>{memory} MiB</dd>
                  <dt className="text-muted-foreground">SMT:</dt>
                  <dd>{smtEnabled ? "Enabled" : "Disabled"}</dd>
                  <dt className="text-muted-foreground">Track Dirty Pages:</dt>
                  <dd>{trackDirtyPages ? "Yes" : "No"}</dd>
                </dl>
              </div>

              <div className="rounded-lg border border-border p-4 space-y-3">
                <h3 className="font-medium">Boot Source</h3>
                <dl className="grid grid-cols-2 gap-2 text-sm">
                  <dt className="text-muted-foreground">Kernel:</dt>
                  <dd className="font-mono text-xs">{kernelPath || "—"}</dd>
                  <dt className="text-muted-foreground">Rootfs:</dt>
                  <dd className="font-mono text-xs">{rootfsPath || "—"}</dd>
                </dl>
              </div>

              <div className="rounded-lg border border-border p-4 space-y-3">
                <h3 className="font-medium">Network</h3>
                <dl className="grid grid-cols-2 gap-2 text-sm">
                  <dt className="text-muted-foreground">Enabled:</dt>
                  <dd>{enableNetwork ? "Yes" : "No"}</dd>
                  {enableNetwork && (
                    <>
                      <dt className="text-muted-foreground">Host Device:</dt>
                      <dd>{hostDevice}</dd>
                      <dt className="text-muted-foreground">Guest MAC:</dt>
                      <dd className="font-mono text-xs">{guestMac || "—"}</dd>
                    </>
                  )}
                </dl>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      <div className="flex justify-between">
        <Button variant="outline" onClick={prevStep} disabled={currentStep === 0}>
          <ChevronLeft className="mr-2 h-4 w-4" />
          Previous
        </Button>
        {currentStep < steps.length - 1 ? (
          <Button onClick={nextStep}>
            Next
            <ChevronRight className="ml-2 h-4 w-4" />
          </Button>
        ) : (
          <Button>Create VM</Button>
        )}
      </div>
    </div>
  )
}
