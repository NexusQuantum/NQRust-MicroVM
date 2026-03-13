"use client"

import { useState, FormEvent } from "react"
import { useActivateLicense, useActivateLicenseFile, useLicenseStatus } from "@/lib/queries"
import { useRouter } from "next/navigation"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Loader2, Key, Upload, CheckCircle2, XCircle, AlertTriangle, ShieldCheck } from "lucide-react"
import { toast } from "sonner"

function formatLicenseKey(raw: string): string {
    const clean = raw.replace(/[^a-zA-Z0-9]/g, "").toUpperCase()
    const groups = clean.match(/.{1,4}/g)
    return (groups?.join("-") || "").slice(0, 19)
}

function isValidKey(key: string): boolean {
    return /^[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$/.test(key)
}

export default function LicenseSetupPage() {
    const router = useRouter()
    const activateMutation = useActivateLicense()
    const activateFileMutation = useActivateLicenseFile()
    const { data: licenseStatus } = useLicenseStatus()

    const [licenseKey, setLicenseKey] = useState("")
    const [fileContent, setFileContent] = useState<string | null>(null)
    const [fileName, setFileName] = useState<string | null>(null)

    function handleKeyChange(value: string) {
        setLicenseKey(formatLicenseKey(value))
    }

    function handleActivateKey(e: FormEvent) {
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
                        setTimeout(() => router.push("/dashboard"), 1200)
                    } else {
                        toast.error(data.error_message || "Activation failed")
                    }
                },
                onError: (err: any) => {
                    toast.error("Activation failed", {
                        description: err?.message || "Network error"
                    })
                }
            }
        )
    }

    function handleActivateFile() {
        if (!fileContent) return
        activateFileMutation.mutate(fileContent, {
            onSuccess: (data) => {
                if (data.is_licensed) {
                    toast.success("Product activated successfully!")
                    setTimeout(() => router.push("/dashboard"), 1200)
                } else {
                    toast.error(data.error_message || "Activation failed")
                }
            },
            onError: (err: any) => {
                toast.error("Activation failed", {
                    description: err?.message || "Network error"
                })
            }
        })
    }

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

    const isActivated = licenseStatus?.is_licensed === true
    const fileIsPending = activateFileMutation.isPending

    return (
        <div className="flex min-h-screen items-center justify-center bg-background p-4">
            <Card className="w-full max-w-lg border-muted bg-card text-card-foreground shadow-xl">
                <CardHeader className="text-center space-y-2">
                    <div className="mx-auto flex h-14 w-14 items-center justify-center rounded-xl bg-primary/10 ring-1 ring-primary/20">
                        <ShieldCheck className="h-7 w-7 text-primary" />
                    </div>
                    <CardTitle className="text-2xl font-bold tracking-tight">Product Activation</CardTitle>
                    <CardDescription className="text-muted-foreground text-sm">
                        Enter your product key or upload an offline license file to activate NQRust-MicroVM.
                    </CardDescription>
                </CardHeader>

                {isActivated ? (
                    <CardContent className="space-y-4 text-center">
                        <div className="flex items-center justify-center gap-2 text-green-500">
                            <CheckCircle2 className="h-6 w-6" />
                            <span className="font-semibold text-lg">Product Activated</span>
                        </div>
                        <div className="space-y-1 text-sm text-muted-foreground">
                            {licenseStatus?.product && <p><strong>Product:</strong> {licenseStatus.product}</p>}
                            {licenseStatus?.customer_name && <p><strong>Customer:</strong> {licenseStatus.customer_name}</p>}
                            {licenseStatus?.expires_at && <p><strong>Expires:</strong> {licenseStatus.expires_at}</p>}
                            {licenseStatus?.license_key && <p><strong>Key:</strong> <code className="font-mono text-xs">{licenseStatus.license_key}</code></p>}
                        </div>
                        <Button onClick={() => router.push("/dashboard")} className="mt-4">
                            Continue to Dashboard
                        </Button>
                    </CardContent>
                ) : (
                    <Tabs defaultValue="key" className="w-full">
                        <TabsList className="grid w-full grid-cols-2 mx-0 rounded-none border-b border-border bg-transparent">
                            <TabsTrigger value="key" className="gap-2 data-[state=active]:border-b-2 data-[state=active]:border-primary rounded-none">
                                <Key className="h-3.5 w-3.5" /> Product Key
                            </TabsTrigger>
                            <TabsTrigger value="file" className="gap-2 data-[state=active]:border-b-2 data-[state=active]:border-primary rounded-none">
                                <Upload className="h-3.5 w-3.5" /> Offline File
                            </TabsTrigger>
                        </TabsList>

                        <TabsContent value="key" className="p-6 pt-4">
                            <form onSubmit={handleActivateKey} className="space-y-4">
                                <div className="space-y-2">
                                    <Label htmlFor="license-key" className="text-sm font-medium">Product Key</Label>
                                    <Input
                                        id="license-key"
                                        type="text"
                                        value={licenseKey}
                                        onChange={(e) => handleKeyChange(e.target.value)}
                                        placeholder="XXXX-XXXX-XXXX-XXXX"
                                        maxLength={19}
                                        autoComplete="off"
                                        spellCheck={false}
                                        className="text-center font-mono text-lg tracking-wider h-12"
                                    />
                                </div>
                                <Button
                                    type="submit"
                                    className="w-full"
                                    disabled={!isValidKey(licenseKey) || activateMutation.isPending}
                                >
                                    {activateMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                                    Activate Product
                                </Button>
                            </form>
                        </TabsContent>

                        <TabsContent value="file" className="p-6 pt-4">
                            <div className="space-y-4">
                                <div
                                    className="flex flex-col items-center justify-center rounded-lg border-2 border-dashed border-muted-foreground/25 p-8 text-center cursor-pointer transition-colors hover:border-primary/50 hover:bg-muted/50"
                                    onClick={() => document.getElementById("lic-file-input")?.click()}
                                    onDragOver={(e) => e.preventDefault()}
                                    onDrop={handleDrop}
                                >
                                    <Upload className="h-8 w-8 text-muted-foreground/50 mb-2" />
                                    <p className="text-sm font-medium">Click to select or drag & drop</p>
                                    <p className="text-xs text-muted-foreground mt-1">Accepts .lic files</p>
                                    <input
                                        type="file"
                                        id="lic-file-input"
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
                                    disabled={!fileContent || fileIsPending}
                                    onClick={handleActivateFile}
                                >
                                    {fileIsPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                                    Upload & Activate
                                </Button>
                                {activateFileMutation.data && !activateFileMutation.data.is_licensed && (
                                    <div className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive border border-destructive/20">
                                        <XCircle className="h-4 w-4 mt-0.5 shrink-0" />
                                        <span>{activateFileMutation.data.error_message || "Activation failed."}</span>
                                    </div>
                                )}
                            </div>
                        </TabsContent>
                    </Tabs>
                )}

                {activateMutation.data && !activateMutation.data.is_licensed && (
                    <CardFooter className="justify-center pb-5">
                        <div className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive border border-destructive/20">
                            <XCircle className="h-4 w-4 mt-0.5 shrink-0" />
                            <span>{activateMutation.data.error_message || "Activation failed."}</span>
                        </div>
                    </CardFooter>
                )}

                {licenseStatus?.is_grace_period && (
                    <CardFooter className="justify-center pb-5">
                        <div className="flex items-start gap-2 rounded-md bg-yellow-500/10 p-3 text-sm text-yellow-600 dark:text-yellow-400 border border-yellow-500/20">
                            <AlertTriangle className="h-4 w-4 mt-0.5 shrink-0" />
                            <span>
                                License in grace period. {licenseStatus.grace_days_remaining} days remaining.
                            </span>
                        </div>
                    </CardFooter>
                )}
            </Card>
        </div>
    )
}
