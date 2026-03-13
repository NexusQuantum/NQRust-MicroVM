"use client"

import { useState, useEffect, FormEvent } from "react"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Loader2, Lock, Globe, ChevronDown, ChevronUp, CheckCircle2, XCircle, AlertTriangle, Upload } from "lucide-react"
import { toast } from "sonner"
import { useLicenseStatus, useActivateLicense, useActivateLicenseFile, useEulaInfo } from "@/lib/queries"
import { EulaContent } from "@/components/license/eula-content"

function formatLicenseKey(raw: string): string {
    const clean = raw.replace(/[^a-zA-Z0-9]/g, "").toUpperCase()
    const groups = clean.match(/.{1,4}/g)
    return (groups?.join("-") || "").slice(0, 19)
}

function isValidKey(key: string): boolean {
    return /^[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$/.test(key)
}

export function LicenseSettingsTab() {
    const { data: license, isLoading } = useLicenseStatus()
    const activateMutation = useActivateLicense()
    const activateFileMutation = useActivateLicenseFile()
    const { data: eulaInfo } = useEulaInfo()

    const [showUpdateForm, setShowUpdateForm] = useState(false)
    const [showFileForm, setShowFileForm] = useState(false)
    const [licenseKey, setLicenseKey] = useState("")
    const [fileContent, setFileContent] = useState<string | null>(null)
    const [fileName, setFileName] = useState<string | null>(null)
    const [language, setLanguage] = useState("en")
    const [eulaContent, setEulaContent] = useState("")
    const [isLoadingEula, setIsLoadingEula] = useState(true)

    // Auto-expand update form when not licensed
    useEffect(() => {
        if (license && !license.is_licensed) {
            setShowUpdateForm(true)
        }
    }, [license])

    function handleFileSelect(e: React.ChangeEvent<HTMLInputElement>) {
        const file = e.target.files?.[0]
        if (file) {
            setFileName(file.name)
            file.text().then(setFileContent)
        }
    }

    function handleDrop(e: React.DragEvent) {
        e.preventDefault()
        const file = e.dataTransfer.files[0]
        if (file && file.name.endsWith(".lic")) {
            setFileName(file.name)
            file.text().then(setFileContent)
        } else {
            toast.error("Please upload a .lic file")
        }
    }

    function handleActivateFile() {
        if (!fileContent) return
        activateFileMutation.mutate(fileContent, {
            onSuccess: (data) => {
                if (data.is_licensed) {
                    toast.success("Offline product activated successfully!")
                    setFileContent(null)
                    setFileName(null)
                    setShowFileForm(false)
                } else {
                    toast.error(data.error_message || "Activation failed")
                }
            },
            onError: (err: any) => {
                toast.error("Activation failed", { description: err?.message || "Network error" })
            },
        })
    }

    // Fetch EULA content on language change
    useEffect(() => {
        setIsLoadingEula(true)
        const fileName = language === "id" ? "EULA_id.md" : "EULA.md"
        fetch(`/eula/${fileName}`)
            .then((res) => {
                if (!res.ok) throw new Error("Could not load EULA")
                return res.text()
            })
            .then((text) => {
                setEulaContent(text)
                setIsLoadingEula(false)
            })
            .catch(() => {
                setEulaContent("# Error loading EULA\n\nPlease contact the administrator.")
                setIsLoadingEula(false)
            })
    }, [language])

    function handleActivate(e: FormEvent) {
        e.preventDefault()
        if (!isValidKey(licenseKey)) {
            toast.error("Invalid format. Use XXXX-XXXX-XXXX-XXXX.")
            return
        }
        activateMutation.mutate(
            { license_key: licenseKey },
            {
                onSuccess: (data) => {
                    if (data.is_licensed) {
                        toast.success("Product activated successfully!")
                        setLicenseKey("")
                        setShowUpdateForm(false)
                    } else {
                        toast.error(data.error_message || "Activation failed")
                    }
                },
                onError: (err: any) => {
                    toast.error("Activation failed", { description: err?.message || "Network error" })
                },
            }
        )
    }

    const statusColor = license?.is_licensed
        ? "text-green-600 dark:text-green-400"
        : license?.is_grace_period
          ? "text-yellow-600 dark:text-yellow-400"
          : "text-red-600 dark:text-red-400"

    const StatusIcon = license?.is_licensed
        ? CheckCircle2
        : license?.is_grace_period
          ? AlertTriangle
          : XCircle

    const statusLabel = license?.status
        ? license.status.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())
        : "Unknown"

    return (
        <div className="space-y-6">
            {/* License Status */}
            <Card>
                <CardHeader>
                    <div className="flex items-center gap-2">
                        <div className="rounded-lg bg-blue-500/10 p-2">
                            <Lock className="h-5 w-5 text-blue-600 dark:text-blue-400" />
                        </div>
                        <div>
                            <CardTitle>Software License</CardTitle>
                            <CardDescription>License status and activation</CardDescription>
                        </div>
                    </div>
                </CardHeader>
                <CardContent className="space-y-4">
                    {isLoading ? (
                        <div className="flex items-center gap-2 text-muted-foreground">
                            <Loader2 className="h-4 w-4 animate-spin" />
                            <span className="text-sm">Loading license status...</span>
                        </div>
                    ) : (
                        <>
                            {/* Status row */}
                            <div className="flex items-center gap-2">
                                <StatusIcon className={`h-5 w-5 ${statusColor}`} />
                                <span className={`font-semibold ${statusColor}`}>{statusLabel}</span>
                            </div>

                            {/* Details grid */}
                            {license?.is_licensed && (
                                <div className="grid grid-cols-2 gap-3 text-sm">
                                    {license.product && (
                                        <div className="rounded-lg border p-3">
                                            <p className="text-muted-foreground text-xs">Product</p>
                                            <p className="font-medium">{license.product}</p>
                                        </div>
                                    )}
                                    {license.customer_name && (
                                        <div className="rounded-lg border p-3">
                                            <p className="text-muted-foreground text-xs">Customer</p>
                                            <p className="font-medium">{license.customer_name}</p>
                                        </div>
                                    )}
                                    {license.license_key && (
                                        <div className="rounded-lg border p-3">
                                            <p className="text-muted-foreground text-xs">License Key</p>
                                            <p className="font-mono text-xs">{license.license_key}</p>
                                        </div>
                                    )}
                                    {license.expires_at && (
                                        <div className="rounded-lg border p-3">
                                            <p className="text-muted-foreground text-xs">Expires</p>
                                            <p className="font-medium">{license.expires_at}</p>
                                        </div>
                                    )}
                                    {license.activations != null && license.max_activations != null && (
                                        <div className="rounded-lg border p-3">
                                            <p className="text-muted-foreground text-xs">Activations</p>
                                            <p className="font-medium">
                                                {license.activations} / {license.max_activations}
                                            </p>
                                        </div>
                                    )}
                                </div>
                            )}

                            {/* Grace period warning */}
                            {license?.is_grace_period && (
                                <div className="rounded-md bg-yellow-500/10 p-3 text-sm text-yellow-600 dark:text-yellow-400 border border-yellow-500/20">
                                    Grace period active.{" "}
                                    {license.grace_days_remaining != null
                                        ? `${license.grace_days_remaining} days remaining until license verification is required.`
                                        : "Please verify your license soon."}
                                </div>
                            )}

                            {/* Update license key */}
                            <div className="border rounded-lg overflow-hidden">
                                <button
                                    type="button"
                                    className="flex w-full items-center justify-between px-4 py-3 text-sm font-medium hover:bg-muted/50 transition-colors"
                                    onClick={() => setShowUpdateForm((v) => !v)}
                                >
                                    <span>Update License Key</span>
                                    {showUpdateForm ? (
                                        <ChevronUp className="h-4 w-4 text-muted-foreground" />
                                    ) : (
                                        <ChevronDown className="h-4 w-4 text-muted-foreground" />
                                    )}
                                </button>
                                {showUpdateForm && (
                                    <form onSubmit={handleActivate} className="border-t px-4 py-4 space-y-3 bg-muted/20">
                                        <div className="space-y-2">
                                            <Label htmlFor="license-key-settings">License Key</Label>
                                            <Input
                                                id="license-key-settings"
                                                type="text"
                                                value={licenseKey}
                                                onChange={(e) => setLicenseKey(formatLicenseKey(e.target.value))}
                                                placeholder="XXXX-XXXX-XXXX-XXXX"
                                                maxLength={19}
                                                autoComplete="off"
                                                spellCheck={false}
                                                className="font-mono text-base tracking-wider text-center"
                                            />
                                        </div>
                                        <Button
                                            type="submit"
                                            disabled={!isValidKey(licenseKey) || activateMutation.isPending}
                                            className="w-full"
                                        >
                                            {activateMutation.isPending && (
                                                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                                            )}
                                            Activate
                                        </Button>
                                        {activateMutation.data && !activateMutation.data.is_licensed && (
                                            <div className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive border border-destructive/20">
                                                <XCircle className="h-4 w-4 mt-0.5 shrink-0" />
                                                <span>{activateMutation.data.error_message || "Activation failed."}</span>
                                            </div>
                                        )}
                                    </form>
                                )}
                            </div>

                            {/* Upload offline license file */}
                            <div className="border rounded-lg overflow-hidden">
                                <button
                                    type="button"
                                    className="flex w-full items-center justify-between px-4 py-3 text-sm font-medium hover:bg-muted/50 transition-colors"
                                    onClick={() => setShowFileForm((v) => !v)}
                                >
                                    <span>Upload Offline License</span>
                                    {showFileForm ? (
                                        <ChevronUp className="h-4 w-4 text-muted-foreground" />
                                    ) : (
                                        <ChevronDown className="h-4 w-4 text-muted-foreground" />
                                    )}
                                </button>
                                {showFileForm && (
                                    <div className="border-t px-4 py-4 space-y-3 bg-muted/20">
                                        <div
                                            className="flex flex-col items-center justify-center rounded-lg border-2 border-dashed border-muted-foreground/25 p-6 text-center cursor-pointer transition-colors hover:border-primary/50 hover:bg-muted/50"
                                            onClick={() => document.getElementById("lic-file-input-settings")?.click()}
                                            onDragOver={(e) => e.preventDefault()}
                                            onDrop={handleDrop}
                                        >
                                            <Upload className="h-6 w-6 text-muted-foreground/50 mb-2" />
                                            <p className="text-sm font-medium">Click to select or drag & drop</p>
                                            <p className="text-xs text-muted-foreground mt-1">Accepts .lic files</p>
                                            <input
                                                type="file"
                                                id="lic-file-input-settings"
                                                accept=".lic"
                                                onChange={handleFileSelect}
                                                className="hidden"
                                            />
                                        </div>
                                        {fileName && (
                                            <p className="text-xs text-muted-foreground text-center">{fileName}</p>
                                        )}
                                        <Button
                                            className="w-full"
                                            disabled={!fileContent || activateFileMutation.isPending}
                                            onClick={handleActivateFile}
                                        >
                                            {activateFileMutation.isPending && (
                                                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                                            )}
                                            Upload & Activate
                                        </Button>
                                        {activateFileMutation.data && !activateFileMutation.data.is_licensed && (
                                            <div className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive border border-destructive/20">
                                                <XCircle className="h-4 w-4 mt-0.5 shrink-0" />
                                                <span>{activateFileMutation.data.error_message || "Activation failed."}</span>
                                            </div>
                                        )}
                                    </div>
                                )}
                            </div>
                        </>
                    )}
                </CardContent>
            </Card>

            {/* EULA */}
            <Card>
                <CardHeader>
                    <div className="flex items-center justify-between">
                        <div>
                            <CardTitle>End User License Agreement</CardTitle>
                            <CardDescription>
                                {eulaInfo?.version && (
                                    <span className="inline-flex items-center rounded-md bg-secondary px-2 py-0.5 text-xs font-medium text-secondary-foreground ring-1 ring-inset ring-secondary-foreground/10">
                                        v{eulaInfo.version}
                                    </span>
                                )}
                            </CardDescription>
                        </div>
                        <div className="flex items-center gap-2">
                            <Globe className="h-4 w-4 text-muted-foreground" />
                            <Select value={language} onValueChange={setLanguage}>
                                <SelectTrigger className="w-[160px] h-8 text-xs">
                                    <SelectValue placeholder="Language" />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="en">English</SelectItem>
                                    <SelectItem value="id">Bahasa Indonesia</SelectItem>
                                </SelectContent>
                            </Select>
                        </div>
                    </div>
                </CardHeader>
                <CardContent className="p-0">
                    <div className="relative border-t bg-muted/20">
                        {isLoadingEula && (
                            <div className="absolute inset-0 flex items-center justify-center bg-background/50 backdrop-blur-sm z-10">
                                <Loader2 className="h-6 w-6 animate-spin text-primary" />
                            </div>
                        )}
                        <ScrollArea className="h-[400px] w-full">
                            <EulaContent content={eulaContent} />
                        </ScrollArea>
                    </div>
                </CardContent>
            </Card>
        </div>
    )
}
