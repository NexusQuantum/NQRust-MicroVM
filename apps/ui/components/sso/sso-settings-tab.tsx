"use client"

import { useState, useEffect } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Switch } from "@/components/ui/switch"
import { Badge } from "@/components/ui/badge"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  DialogDescription,
} from "@/components/ui/dialog"
import {
  ssoApi,
  type SsoProviderConfig,
  type CreateSsoProviderRequest,
  type SsoTestResult,
} from "@/lib/api/sso"
import { toast } from "sonner"
import {
  Plus,
  Trash2,
  TestTube,
  Shield,
  ShieldOff,
  Eye,
  Pencil,
  CheckCircle2,
  XCircle,
} from "lucide-react"
import { useAuthStore } from "@/lib/auth/store"
import { ProviderForm } from "./provider-form"

export function SsoSettingsTab() {
  const { user } = useAuthStore()
  const isAdmin = user?.role === "admin"

  const [providers, setProviders] = useState<SsoProviderConfig[]>([])
  const [loading, setLoading] = useState(true)
  const [createOpen, setCreateOpen] = useState(false)
  const [editProvider, setEditProvider] = useState<SsoProviderConfig | null>(null)
  const [viewProvider, setViewProvider] = useState<SsoProviderConfig | null>(null)
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
    if (isAdmin) {
      loadProviders()
    } else {
      setLoading(false)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isAdmin])

  if (!isAdmin) {
    return (
      <Card>
        <CardContent className="flex flex-col items-center justify-center py-12">
          <ShieldOff className="h-12 w-12 text-muted-foreground mb-4" />
          <p className="text-muted-foreground font-medium">Admin access required</p>
          <p className="text-sm text-muted-foreground mt-1">
            Only administrators can configure SSO providers.
          </p>
        </CardContent>
      </Card>
    )
  }

  const handleToggle = async (id: string, enabled: boolean) => {
    try {
      await ssoApi.update(id, { enabled })
      setProviders((prev) => prev.map((p) => (p.id === id ? { ...p, enabled } : p)))
      toast.success(`Provider ${enabled ? "enabled" : "disabled"}`)
    } catch {
      toast.error("Failed to update provider")
    }
  }

  const handleDelete = async (id: string, name: string) => {
    if (
      !confirm(
        `Delete SSO provider "${name}"? This will remove all linked user identities.`
      )
    )
      return
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
      setCreateOpen(false)
      loadProviders()
      toast.success("SSO provider created")
    } catch {
      toast.error("Failed to create provider")
    }
  }

  const handleUpdate = async (req: CreateSsoProviderRequest) => {
    if (!editProvider) return
    try {
      // Only send the fields that the update endpoint accepts.
      // Slug and protocol are locked in edit mode, so we don't need to send them,
      // but the type requires them — the backend ignores unchanged fields.
      const { slug: _slug, protocol: _protocol, ...updatable } = req
      void _slug
      void _protocol
      await ssoApi.update(editProvider.id, updatable)
      setEditProvider(null)
      loadProviders()
      toast.success("SSO provider updated")
    } catch {
      toast.error("Failed to update provider")
    }
  }

  if (loading) {
    return (
      <Card>
        <CardContent className="flex items-center justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-orange-600" />
        </CardContent>
      </Card>
    )
  }

  return (
    <>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <div className="rounded-lg bg-violet-500/10 p-2">
                <Shield className="h-5 w-5 text-violet-600 dark:text-violet-400" />
              </div>
              <div>
                <CardTitle>Identity Providers</CardTitle>
                <CardDescription>
                  Configure SAML and OAuth/OIDC providers for single sign-on
                </CardDescription>
              </div>
            </div>
            <Dialog open={createOpen} onOpenChange={setCreateOpen}>
              <DialogTrigger asChild>
                <Button>
                  <Plus className="mr-2 h-4 w-4" />
                  Add Provider
                </Button>
              </DialogTrigger>
              <DialogContent className="max-w-lg max-h-[90vh] overflow-y-auto">
                <DialogHeader>
                  <DialogTitle>Add SSO Provider</DialogTitle>
                  <DialogDescription>
                    Register an OIDC or SAML identity provider. Required fields are
                    marked with *.
                  </DialogDescription>
                </DialogHeader>
                <ProviderForm mode="create" onSubmit={handleCreate} />
              </DialogContent>
            </Dialog>
          </div>
        </CardHeader>
        <CardContent>
          {providers.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 border border-dashed rounded-lg">
              <Shield className="h-12 w-12 text-muted-foreground mb-4" />
              <p className="text-muted-foreground">No SSO providers configured</p>
              <p className="text-sm text-muted-foreground mt-1">
                Add an OIDC or SAML provider to enable single sign-on
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {providers.map((provider) => (
                <div key={provider.id} className="rounded-lg border p-4 space-y-3">
                  <div className="flex items-start justify-between gap-4">
                    <div className="space-y-1 min-w-0 flex-1">
                      <div className="flex items-center gap-2 flex-wrap">
                        <span className="font-medium">{provider.name}</span>
                        <Badge
                          variant={provider.protocol === "oidc" ? "default" : "secondary"}
                        >
                          {provider.protocol.toUpperCase()}
                        </Badge>
                        {!provider.enabled && <Badge variant="outline">Disabled</Badge>}
                      </div>
                      <p className="text-sm text-muted-foreground truncate">
                        Slug: <span className="font-mono">{provider.slug}</span>
                        {provider.protocol === "oidc" && provider.oidc_issuer_url && (
                          <> · Issuer: {provider.oidc_issuer_url}</>
                        )}
                        {provider.protocol === "saml" && provider.saml_idp_entity_id && (
                          <> · IdP: {provider.saml_idp_entity_id}</>
                        )}
                      </p>
                    </div>
                    <div className="flex items-center gap-2 shrink-0 flex-wrap justify-end">
                      <Switch
                        checked={provider.enabled}
                        onCheckedChange={(checked) => handleToggle(provider.id, checked)}
                      />
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => setViewProvider(provider)}
                      >
                        <Eye className="h-4 w-4 mr-1" />
                        View
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => setEditProvider(provider)}
                      >
                        <Pencil className="h-4 w-4 mr-1" />
                        Edit
                      </Button>
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
                  </div>
                  <div className="grid grid-cols-2 md:grid-cols-3 gap-3 text-sm pt-2 border-t">
                    <div>
                      <span className="text-muted-foreground">Default role:</span>{" "}
                      <span className="font-medium capitalize">{provider.default_role}</span>
                    </div>
                    <div>
                      <span className="text-muted-foreground">JIT:</span>{" "}
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
                    <div
                      className={`flex items-start gap-2 p-2 rounded text-sm ${
                        testResults[provider.id].success
                          ? "bg-green-500/10 text-green-600 dark:text-green-400"
                          : "bg-red-500/10 text-red-600 dark:text-red-400"
                      }`}
                    >
                      {testResults[provider.id].success ? (
                        <CheckCircle2 className="h-4 w-4 shrink-0 mt-0.5" />
                      ) : (
                        <XCircle className="h-4 w-4 shrink-0 mt-0.5" />
                      )}
                      <span>
                        {testResults[provider.id].success
                          ? testResults[provider.id].message
                          : testResults[provider.id].error}
                      </span>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Edit dialog */}
      <Dialog
        open={!!editProvider}
        onOpenChange={(open) => !open && setEditProvider(null)}
      >
        <DialogContent className="max-w-lg max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Edit SSO Provider</DialogTitle>
            <DialogDescription>
              Update the configuration for{" "}
              <strong>{editProvider?.name}</strong>. The slug and protocol cannot
              be changed.
            </DialogDescription>
          </DialogHeader>
          {editProvider && (
            <ProviderForm
              mode="edit"
              initialConfig={editProvider}
              onSubmit={handleUpdate}
            />
          )}
        </DialogContent>
      </Dialog>

      {/* View dialog */}
      <Dialog
        open={!!viewProvider}
        onOpenChange={(open) => !open && setViewProvider(null)}
      >
        <DialogContent className="max-w-lg max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{viewProvider?.name}</DialogTitle>
            <DialogDescription>Provider configuration (read-only)</DialogDescription>
          </DialogHeader>
          {viewProvider && <ProviderDetails provider={viewProvider} />}
        </DialogContent>
      </Dialog>
    </>
  )
}

// ─── Read-only details view ─────────────────────────────────────────

function ProviderDetails({ provider }: { provider: SsoProviderConfig }) {
  const callbackUrl =
    typeof window !== "undefined"
      ? `${window.location.protocol}//${window.location.host}/v1/sso/${provider.protocol}/${provider.slug}/${provider.protocol === "oidc" ? "callback" : "acs"}`
      : `/v1/sso/${provider.protocol}/${provider.slug}/${provider.protocol === "oidc" ? "callback" : "acs"}`

  const mapping = provider.role_mapping || {}

  return (
    <div className="space-y-4 text-sm">
      <Section title="General">
        <Field label="Protocol">
          <Badge variant={provider.protocol === "oidc" ? "default" : "secondary"}>
            {provider.protocol.toUpperCase()}
          </Badge>
        </Field>
        <Field label="Status">
          <Badge variant={provider.enabled ? "default" : "outline"}>
            {provider.enabled ? "Enabled" : "Disabled"}
          </Badge>
        </Field>
        <Field label="Slug" mono>
          {provider.slug}
        </Field>
        <Field label="Icon Hint">{provider.icon_hint ?? "generic"}</Field>
        <Field label="Display Order">{provider.display_order}</Field>
      </Section>

      {provider.protocol === "oidc" ? (
        <Section title="OIDC Configuration">
          <Field label="Issuer URL" mono breakAll>
            {provider.oidc_issuer_url ?? "—"}
          </Field>
          <Field label="Client ID" mono breakAll>
            {provider.oidc_client_id ?? "—"}
          </Field>
          <Field label="Client Secret">
            {provider.oidc_secret_set ? (
              <Badge variant="default">Set</Badge>
            ) : (
              <Badge variant="outline">Not set</Badge>
            )}
          </Field>
          <Field label="Scopes" mono>
            {provider.oidc_scopes ?? "openid profile email"}
          </Field>
        </Section>
      ) : (
        <Section title="SAML Configuration">
          <Field label="IdP Entity ID" mono breakAll>
            {provider.saml_idp_entity_id ?? "—"}
          </Field>
          <Field label="IdP SSO URL" mono breakAll>
            {provider.saml_idp_sso_url ?? "—"}
          </Field>
          <Field label="SP Entity ID" mono breakAll>
            {provider.saml_sp_entity_id ?? "—"}
          </Field>
        </Section>
      )}

      <Section title="Role Mapping">
        <Field label="Claim Name" mono>
          {provider.role_claim_name ?? "groups"}
        </Field>
        <Field label="Default Role">
          <span className="capitalize">{provider.default_role}</span>
        </Field>
        <Field label="JIT Provisioning">
          {provider.allow_jit_provisioning ? "Enabled" : "Disabled"}
        </Field>
        {(["admin", "user", "viewer"] as const).map((role) => {
          const groups = mapping[role]
          if (!Array.isArray(groups) || groups.length === 0) return null
          return (
            <Field
              key={role}
              label={
                <span className="flex items-center gap-1">
                  <Badge
                    variant={
                      role === "admin"
                        ? "default"
                        : role === "user"
                        ? "secondary"
                        : "outline"
                    }
                    className="h-5"
                  >
                    {role}
                  </Badge>
                </span>
              }
            >
              <div className="flex flex-wrap gap-1">
                {(groups as string[]).map((g) => (
                  <code
                    key={g}
                    className="text-xs bg-muted px-1.5 py-0.5 rounded"
                  >
                    {g}
                  </code>
                ))}
              </div>
            </Field>
          )
        })}
      </Section>

      <Section title="Callback URL">
        <p className="text-xs text-muted-foreground mb-1">
          Register this URL in your IdP as the{" "}
          {provider.protocol === "oidc"
            ? "redirect URI"
            : "Assertion Consumer Service URL"}
          :
        </p>
        <code className="block text-xs bg-muted p-2 rounded break-all">
          {callbackUrl}
        </code>
      </Section>

      <Section title="Timestamps">
        <Field label="Created">
          {new Date(provider.created_at).toLocaleString()}
        </Field>
        <Field label="Updated">
          {new Date(provider.updated_at).toLocaleString()}
        </Field>
      </Section>
    </div>
  )
}

function Section({
  title,
  children,
}: {
  title: string
  children: React.ReactNode
}) {
  return (
    <div className="space-y-2">
      <h4 className="text-xs font-semibold uppercase text-muted-foreground">
        {title}
      </h4>
      <div className="space-y-2 border rounded-lg p-3">{children}</div>
    </div>
  )
}

function Field({
  label,
  children,
  mono,
  breakAll,
}: {
  label: React.ReactNode
  children: React.ReactNode
  mono?: boolean
  breakAll?: boolean
}) {
  return (
    <div className="grid grid-cols-3 gap-2 items-start">
      <div className="text-xs text-muted-foreground col-span-1">{label}</div>
      <div
        className={`col-span-2 text-sm ${mono ? "font-mono" : ""} ${
          breakAll ? "break-all" : ""
        }`}
      >
        {children}
      </div>
    </div>
  )
}
