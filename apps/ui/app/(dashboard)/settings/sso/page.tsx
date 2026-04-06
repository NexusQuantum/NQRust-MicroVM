"use client"

import { useState, useEffect } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { Badge } from "@/components/ui/badge"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { ssoApi, type SsoProviderConfig, type CreateSsoProviderRequest, type SsoTestResult } from "@/lib/api/sso"
import { toast } from "sonner"
import { Plus, Trash2, TestTube, Shield, ArrowLeft } from "lucide-react"
import Link from "next/link"

export default function SsoSettingsPage() {
  const [providers, setProviders] = useState<SsoProviderConfig[]>([])
  const [loading, setLoading] = useState(true)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [testResults, setTestResults] = useState<Record<string, SsoTestResult>>({})

  const loadProviders = async () => {
    try {
      const items = await ssoApi.listAll()
      setProviders(items)
    } catch {
      toast.error("Failed to load SSO providers")
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadProviders()
  }, [])

  const handleToggle = async (id: string, enabled: boolean) => {
    try {
      await ssoApi.update(id, { enabled })
      setProviders((prev) =>
        prev.map((p) => (p.id === id ? { ...p, enabled } : p))
      )
      toast.success(`Provider ${enabled ? "enabled" : "disabled"}`)
    } catch {
      toast.error("Failed to update provider")
    }
  }

  const handleDelete = async (id: string, name: string) => {
    if (!confirm(`Delete SSO provider "${name}"? This will remove all linked user identities.`)) return
    try {
      await ssoApi.delete(id)
      setProviders((prev) => prev.filter((p) => p.id !== id))
      toast.success("Provider deleted")
    } catch {
      toast.error("Failed to delete provider")
    }
  }

  const handleTest = async (id: string) => {
    try {
      const result = await ssoApi.test(id)
      setTestResults((prev) => ({ ...prev, [id]: result }))
      if (result.success) {
        toast.success(result.message || "Connection successful")
      } else {
        toast.error(result.error || "Connection failed")
      }
    } catch {
      toast.error("Failed to test provider")
    }
  }

  const handleCreate = async (req: CreateSsoProviderRequest) => {
    try {
      await ssoApi.create(req)
      setDialogOpen(false)
      loadProviders()
      toast.success("SSO provider created")
    } catch {
      toast.error("Failed to create provider")
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center p-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-orange-600" />
      </div>
    )
  }

  return (
    <div className="space-y-6 p-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Link href="/settings">
            <Button variant="ghost" size="icon">
              <ArrowLeft className="h-4 w-4" />
            </Button>
          </Link>
          <div>
            <h1 className="text-2xl font-bold">SSO Configuration</h1>
            <p className="text-muted-foreground">
              Configure SAML and OAuth/OIDC identity providers
            </p>
          </div>
        </div>
        <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
          <DialogTrigger asChild>
            <Button>
              <Plus className="mr-2 h-4 w-4" />
              Add Provider
            </Button>
          </DialogTrigger>
          <DialogContent className="max-w-lg">
            <DialogHeader>
              <DialogTitle>Add SSO Provider</DialogTitle>
            </DialogHeader>
            <CreateProviderForm onSubmit={handleCreate} />
          </DialogContent>
        </Dialog>
      </div>

      {providers.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12">
            <Shield className="h-12 w-12 text-muted-foreground mb-4" />
            <p className="text-muted-foreground">No SSO providers configured</p>
            <p className="text-sm text-muted-foreground mt-1">
              Add an OIDC or SAML provider to enable single sign-on
            </p>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-4">
          {providers.map((provider) => (
            <Card key={provider.id}>
              <CardHeader className="flex flex-row items-center justify-between pb-2">
                <div className="space-y-1">
                  <CardTitle className="text-lg flex items-center gap-2">
                    {provider.name}
                    <Badge variant={provider.protocol === "oidc" ? "default" : "secondary"}>
                      {provider.protocol.toUpperCase()}
                    </Badge>
                    {!provider.enabled && (
                      <Badge variant="outline">Disabled</Badge>
                    )}
                  </CardTitle>
                  <CardDescription>
                    Slug: {provider.slug}
                    {provider.protocol === "oidc" && provider.oidc_issuer_url && (
                      <> | Issuer: {provider.oidc_issuer_url}</>
                    )}
                    {provider.protocol === "saml" && provider.saml_idp_entity_id && (
                      <> | IdP: {provider.saml_idp_entity_id}</>
                    )}
                  </CardDescription>
                </div>
                <div className="flex items-center gap-2">
                  <Switch
                    checked={provider.enabled}
                    onCheckedChange={(checked) => handleToggle(provider.id, checked)}
                  />
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => handleTest(provider.id)}
                  >
                    <TestTube className="h-4 w-4 mr-1" />
                    Test
                  </Button>
                  <Button
                    variant="destructive"
                    size="sm"
                    onClick={() => handleDelete(provider.id, provider.name)}
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              </CardHeader>
              <CardContent>
                <div className="grid grid-cols-3 gap-4 text-sm">
                  <div>
                    <span className="text-muted-foreground">Default Role:</span>{" "}
                    <span className="font-medium">{provider.default_role}</span>
                  </div>
                  <div>
                    <span className="text-muted-foreground">JIT Provisioning:</span>{" "}
                    <span className="font-medium">
                      {provider.allow_jit_provisioning ? "Enabled" : "Disabled"}
                    </span>
                  </div>
                  <div>
                    <span className="text-muted-foreground">Secret:</span>{" "}
                    <span className="font-medium">
                      {provider.oidc_secret_set ? "Set" : "Not set"}
                    </span>
                  </div>
                </div>
                {testResults[provider.id] && (
                  <div className={`mt-3 p-2 rounded text-sm ${testResults[provider.id].success ? "bg-green-500/10 text-green-500" : "bg-red-500/10 text-red-500"}`}>
                    {testResults[provider.id].success
                      ? testResults[provider.id].message
                      : testResults[provider.id].error}
                  </div>
                )}
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  )
}

function CreateProviderForm({ onSubmit }: { onSubmit: (req: CreateSsoProviderRequest) => void }) {
  const [protocol, setProtocol] = useState<"oidc" | "saml">("oidc")
  const [name, setName] = useState("")
  const [slug, setSlug] = useState("")
  const [issuerUrl, setIssuerUrl] = useState("")
  const [clientId, setClientId] = useState("")
  const [clientSecret, setClientSecret] = useState("")
  const [idpSsoUrl, setIdpSsoUrl] = useState("")
  const [idpEntityId, setIdpEntityId] = useState("")
  const [idpCertPem, setIdpCertPem] = useState("")
  const [defaultRole, setDefaultRole] = useState<"admin" | "user" | "viewer">("viewer")

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    const req: CreateSsoProviderRequest = {
      name,
      slug: slug || name.toLowerCase().replace(/[^a-z0-9]+/g, "-"),
      protocol,
      default_role: defaultRole,
    }
    if (protocol === "oidc") {
      req.oidc_issuer_url = issuerUrl
      req.oidc_client_id = clientId
      if (clientSecret) req.oidc_client_secret = clientSecret
    } else {
      if (idpSsoUrl) req.saml_idp_sso_url = idpSsoUrl
      if (idpEntityId) req.saml_idp_entity_id = idpEntityId
      if (idpCertPem) req.saml_idp_certificate_pem = idpCertPem
    }
    onSubmit(req)
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div className="space-y-2">
        <Label>Protocol</Label>
        <Select value={protocol} onValueChange={(v) => setProtocol(v as "oidc" | "saml")}>
          <SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="oidc">OIDC (OAuth2/OpenID Connect)</SelectItem>
            <SelectItem value="saml">SAML 2.0</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label>Name</Label>
          <Input placeholder="Corporate Okta" value={name} onChange={(e) => setName(e.target.value)} required />
        </div>
        <div className="space-y-2">
          <Label>Slug</Label>
          <Input placeholder="corporate-okta" value={slug} onChange={(e) => setSlug(e.target.value)} />
        </div>
      </div>

      {protocol === "oidc" ? (
        <>
          <div className="space-y-2">
            <Label>Issuer URL</Label>
            <Input placeholder="https://accounts.google.com" value={issuerUrl} onChange={(e) => setIssuerUrl(e.target.value)} required />
          </div>
          <div className="space-y-2">
            <Label>Client ID</Label>
            <Input value={clientId} onChange={(e) => setClientId(e.target.value)} required />
          </div>
          <div className="space-y-2">
            <Label>Client Secret</Label>
            <Input type="password" value={clientSecret} onChange={(e) => setClientSecret(e.target.value)} />
          </div>
        </>
      ) : (
        <>
          <div className="space-y-2">
            <Label>IdP SSO URL</Label>
            <Input placeholder="https://idp.example.com/saml/sso" value={idpSsoUrl} onChange={(e) => setIdpSsoUrl(e.target.value)} required />
          </div>
          <div className="space-y-2">
            <Label>IdP Entity ID</Label>
            <Input value={idpEntityId} onChange={(e) => setIdpEntityId(e.target.value)} />
          </div>
          <div className="space-y-2">
            <Label>IdP Certificate (PEM)</Label>
            <textarea
              className="w-full min-h-[80px] rounded-md border bg-background px-3 py-2 text-sm font-mono"
              placeholder="-----BEGIN CERTIFICATE-----"
              value={idpCertPem}
              onChange={(e) => setIdpCertPem(e.target.value)}
            />
          </div>
        </>
      )}

      <div className="space-y-2">
        <Label>Default Role for New Users</Label>
        <Select value={defaultRole} onValueChange={(v) => setDefaultRole(v as "admin" | "user" | "viewer")}>
          <SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="viewer">Viewer (read-only)</SelectItem>
            <SelectItem value="user">User (create resources)</SelectItem>
            <SelectItem value="admin">Admin (full access)</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <Button type="submit" className="w-full">Create Provider</Button>
    </form>
  )
}
