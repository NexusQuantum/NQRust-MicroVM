"use client"

import { useState, useEffect, useMemo } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { Badge } from "@/components/ui/badge"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Copy, Check, ExternalLink, Info } from "lucide-react"
import { toast } from "sonner"
import type { CreateSsoProviderRequest, SsoProviderConfig } from "@/lib/api/sso"

// ─── Presets ────────────────────────────────────────────────────────

export type Preset = "keycloak" | "google" | "azure" | "okta" | "custom"

export interface PresetConfig {
  label: string
  iconHint: string
  roleClaimName: string
  scopes?: string
  issuerHint?: string
  note?: string
  /** Step-by-step setup instructions shown in the create form. */
  setupSteps?: string[]
  /** URL where admins manage their IdP client. */
  docsUrl?: string
}

export const PRESETS: Record<Preset, PresetConfig> = {
  keycloak: {
    label: "Keycloak",
    iconHint: "keycloak",
    roleClaimName: "realm_access.roles",
    scopes: "openid profile email roles",
    issuerHint: "https://keycloak.example.com/realms/YOUR_REALM",
    note: "Keycloak exposes realm roles at the nested claim realm_access.roles. Make sure your client has the 'roles' scope enabled.",
    setupSteps: [
      "In Keycloak, go to Clients → Create client",
      "General settings: set Client type to OpenID Connect, enter a Client ID (e.g. nexus-microvm)",
      "Capability config: turn ON 'Client authentication' and 'Standard flow', leave the rest OFF",
      "Login settings: paste the Valid redirect URI shown below",
      "Save, then open the Credentials tab and copy the Client Secret",
      "Paste the Client ID + Secret into this form along with the realm issuer URL",
    ],
    docsUrl: "https://www.keycloak.org/docs/latest/server_admin/#_oidc_clients",
  },
  google: {
    label: "Google Workspace",
    iconHint: "google",
    roleClaimName: "groups",
    scopes: "openid profile email",
    issuerHint: "https://accounts.google.com",
    note: "To use Google Workspace groups, enable Google Groups in your OAuth consent screen.",
    setupSteps: [
      "Open Google Cloud Console → APIs & Services → Credentials",
      "Create OAuth 2.0 Client ID (Application type: Web application)",
      "Paste the Authorized redirect URI shown below",
      "Copy the Client ID and Client Secret into this form",
      "(Optional) Enable the Admin SDK API and configure a 'groups' custom claim to use role mapping",
    ],
    docsUrl: "https://developers.google.com/identity/protocols/oauth2/openid-connect",
  },
  azure: {
    label: "Azure AD / Entra ID",
    iconHint: "microsoft",
    roleClaimName: "roles,groups",
    scopes: "openid profile email",
    issuerHint: "https://login.microsoftonline.com/{tenant-id}/v2.0",
    note: "Azure AD sends app roles under 'roles' and security groups under 'groups'.",
    setupSteps: [
      "In Entra ID admin center, go to App registrations → New registration",
      "Set Redirect URI type to Web and paste the URL shown below",
      "After creation, go to Certificates & secrets → Client secrets → New client secret, copy the value",
      "In Token configuration, optionally add the 'groups' claim for role mapping",
      "Copy the Application (client) ID and Directory (tenant) ID — the Issuer URL uses the tenant ID",
    ],
    docsUrl: "https://learn.microsoft.com/en-us/entra/identity-platform/quickstart-register-app",
  },
  okta: {
    label: "Okta",
    iconHint: "okta",
    roleClaimName: "groups",
    scopes: "openid profile email groups",
    issuerHint: "https://{yourOktaDomain}/oauth2/default",
    note: "Ensure a 'groups' claim is configured on your Okta authorization server.",
    setupSteps: [
      "In Okta Admin, go to Applications → Create App Integration → OIDC - Web Application",
      "Paste the Sign-in redirect URI shown below",
      "After creation, copy the Client ID and Client Secret from the General tab",
      "In Security → API → Authorization Servers → default → Claims, add a 'groups' claim mapped to Groups (Matches regex: .*)",
    ],
    docsUrl: "https://developer.okta.com/docs/guides/implement-grant-type/authcode/main/",
  },
  custom: {
    label: "Custom OIDC",
    iconHint: "generic",
    roleClaimName: "groups",
    scopes: "openid profile email",
    setupSteps: [
      "Register an OIDC client in your identity provider",
      "Grant type: Authorization Code with PKCE",
      "Paste the redirect URI shown below into your IdP's client config",
      "Copy the Client ID and Client Secret back into this form",
      "Ensure your IdP issues a claim containing groups/roles for role mapping",
    ],
  },
}

// ─── Validation ─────────────────────────────────────────────────────

const SLUG_RE = /^[a-z0-9]+(?:-[a-z0-9]+)*$/
const URL_RE = /^https?:\/\/[^\s/$.?#].[^\s]*$/i

interface FieldErrors {
  name?: string
  slug?: string
  issuerUrl?: string
  clientId?: string
  idpSsoUrl?: string
  roleClaimName?: string
}

// ─── Form State ─────────────────────────────────────────────────────

export interface ProviderFormValues {
  preset: Preset
  protocol: "oidc" | "saml"
  name: string
  slug: string
  issuerUrl: string
  clientId: string
  clientSecret: string
  scopes: string
  idpSsoUrl: string
  idpEntityId: string
  idpCertPem: string
  defaultRole: "admin" | "user" | "viewer"
  allowJit: boolean
  roleClaimName: string
  adminGroups: string
  userGroups: string
  viewerGroups: string
}

function emptyValues(): ProviderFormValues {
  return {
    preset: "custom",
    protocol: "oidc",
    name: "",
    slug: "",
    issuerUrl: "",
    clientId: "",
    clientSecret: "",
    scopes: "openid profile email",
    idpSsoUrl: "",
    idpEntityId: "",
    idpCertPem: "",
    defaultRole: "viewer",
    allowJit: true,
    roleClaimName: "groups",
    adminGroups: "",
    userGroups: "",
    viewerGroups: "",
  }
}

/** Populate form values from an existing provider config (for edit mode). */
function valuesFromConfig(config: SsoProviderConfig): ProviderFormValues {
  // Detect preset from icon_hint if possible, otherwise default to custom
  const preset: Preset = (Object.keys(PRESETS) as Preset[]).find(
    (p) => PRESETS[p].iconHint === config.icon_hint
  ) ?? "custom"

  const mapping = config.role_mapping || {}
  const groupsAsString = (role: "admin" | "user" | "viewer") =>
    Array.isArray(mapping[role]) ? (mapping[role] as string[]).join(", ") : ""

  return {
    preset,
    protocol: config.protocol,
    name: config.name,
    slug: config.slug,
    issuerUrl: config.oidc_issuer_url ?? "",
    clientId: config.oidc_client_id ?? "",
    clientSecret: "", // secret is write-only; never sent back
    scopes: config.oidc_scopes ?? "openid profile email",
    idpSsoUrl: config.saml_idp_sso_url ?? "",
    idpEntityId: config.saml_idp_entity_id ?? "",
    idpCertPem: "",
    defaultRole: config.default_role,
    allowJit: config.allow_jit_provisioning,
    roleClaimName: config.role_claim_name ?? "groups",
    adminGroups: groupsAsString("admin"),
    userGroups: groupsAsString("user"),
    viewerGroups: groupsAsString("viewer"),
  }
}

// ─── Component ──────────────────────────────────────────────────────

interface ProviderFormProps {
  mode: "create" | "edit"
  /** Existing config (required for edit mode). */
  initialConfig?: SsoProviderConfig
  /** Called on valid submit — returns the payload for create or update. */
  onSubmit: (req: CreateSsoProviderRequest) => Promise<void> | void
  submitLabel?: string
}

export function ProviderForm({
  mode,
  initialConfig,
  onSubmit,
  submitLabel,
}: ProviderFormProps) {
  const [v, setV] = useState<ProviderFormValues>(() =>
    mode === "edit" && initialConfig ? valuesFromConfig(initialConfig) : emptyValues()
  )
  const [touched, setTouched] = useState<Record<string, boolean>>({})
  const [submitting, setSubmitting] = useState(false)
  const [submitAttempted, setSubmitAttempted] = useState(false)

  // Reset when initialConfig changes (e.g. opening edit dialog for a different provider)
  useEffect(() => {
    if (mode === "edit" && initialConfig) {
      setV(valuesFromConfig(initialConfig))
      setTouched({})
      setSubmitAttempted(false)
    }
  }, [mode, initialConfig])

  const update = <K extends keyof ProviderFormValues>(
    key: K,
    value: ProviderFormValues[K]
  ) => {
    setV((prev) => ({ ...prev, [key]: value }))
  }

  const markTouched = (key: string) =>
    setTouched((prev) => ({ ...prev, [key]: true }))

  // ─── Validation ───────────────────────────────────────────────────
  const errors: FieldErrors = useMemo(() => {
    const e: FieldErrors = {}
    if (!v.name.trim()) e.name = "Name is required"
    if (!v.slug.trim()) {
      e.slug = "Slug is required"
    } else if (!SLUG_RE.test(v.slug)) {
      e.slug = "Lowercase letters, numbers, and hyphens only (e.g. corporate-keycloak)"
    }
    if (!v.roleClaimName.trim()) {
      e.roleClaimName = "Role claim name is required"
    }

    if (v.protocol === "oidc") {
      if (!v.issuerUrl.trim()) {
        e.issuerUrl = "Issuer URL is required"
      } else if (!URL_RE.test(v.issuerUrl.trim())) {
        e.issuerUrl = "Must be a valid http(s):// URL"
      }
      if (!v.clientId.trim()) e.clientId = "Client ID is required"
    } else {
      if (!v.idpSsoUrl.trim()) {
        e.idpSsoUrl = "IdP SSO URL is required"
      } else if (!URL_RE.test(v.idpSsoUrl.trim())) {
        e.idpSsoUrl = "Must be a valid http(s):// URL"
      }
    }
    return e
  }, [v])

  const isValid = Object.keys(errors).length === 0

  const showError = (field: keyof FieldErrors) =>
    (touched[field] || submitAttempted) && errors[field]

  // ─── Presets ──────────────────────────────────────────────────────
  const applyPreset = (p: Preset) => {
    if (mode === "edit") return // can't change preset on edit
    update("preset", p)
    if (p === "custom") return
    const cfg = PRESETS[p]
    update("protocol", "oidc")
    update("roleClaimName", cfg.roleClaimName)
    if (cfg.scopes) update("scopes", cfg.scopes)
    if (!v.name) update("name", cfg.label)
  }

  // ─── Live callback URL preview ───────────────────────────────────
  const callbackUrl = useMemo(() => {
    if (typeof window === "undefined") return ""
    const slug = v.slug.trim() || "<slug>"
    const path =
      v.protocol === "oidc"
        ? `/v1/sso/oidc/${slug}/callback`
        : `/v1/sso/saml/${slug}/acs`
    return `${window.location.protocol}//${window.location.host}${path}`
  }, [v.slug, v.protocol])

  // Auto-generate slug from name on create mode
  useEffect(() => {
    if (mode === "create" && !touched.slug && v.name) {
      const auto = v.name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "")
      setV((prev) => ({ ...prev, slug: auto }))
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [v.name, mode])

  // ─── Submit ───────────────────────────────────────────────────────
  const parseGroupList = (input: string): string[] =>
    input
      .split(/[\n,]/)
      .map((s) => s.trim())
      .filter((s) => s.length > 0)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setSubmitAttempted(true)
    if (!isValid) return

    const mapping: Record<string, string[]> = {}
    const adminList = parseGroupList(v.adminGroups)
    const userList = parseGroupList(v.userGroups)
    const viewerList = parseGroupList(v.viewerGroups)
    if (adminList.length > 0) mapping.admin = adminList
    if (userList.length > 0) mapping.user = userList
    if (viewerList.length > 0) mapping.viewer = viewerList

    const req: CreateSsoProviderRequest = {
      name: v.name.trim(),
      slug: v.slug.trim(),
      protocol: v.protocol,
      default_role: v.defaultRole,
      allow_jit_provisioning: v.allowJit,
      role_claim_name: v.roleClaimName.trim() || "groups",
      role_mapping: mapping,
      icon_hint: PRESETS[v.preset].iconHint,
    }
    if (v.protocol === "oidc") {
      req.oidc_issuer_url = v.issuerUrl.trim()
      req.oidc_client_id = v.clientId.trim()
      // For edit mode, only send the secret if the user typed a new one.
      // For create mode, always send if provided.
      if (v.clientSecret) req.oidc_client_secret = v.clientSecret
      if (v.scopes) req.oidc_scopes = v.scopes.trim()
    } else {
      if (v.idpSsoUrl) req.saml_idp_sso_url = v.idpSsoUrl.trim()
      if (v.idpEntityId) req.saml_idp_entity_id = v.idpEntityId.trim()
      if (v.idpCertPem) req.saml_idp_certificate_pem = v.idpCertPem
    }

    try {
      setSubmitting(true)
      await onSubmit(req)
    } finally {
      setSubmitting(false)
    }
  }

  // ─── Render ───────────────────────────────────────────────────────
  const isEdit = mode === "edit"

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      {/* Preset (create only) */}
      {!isEdit && (
        <div className="space-y-2">
          <Label>Preset</Label>
          <Select value={v.preset} onValueChange={(val) => applyPreset(val as Preset)}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="keycloak">Keycloak</SelectItem>
              <SelectItem value="google">Google Workspace</SelectItem>
              <SelectItem value="azure">Azure AD / Entra ID</SelectItem>
              <SelectItem value="okta">Okta</SelectItem>
              <SelectItem value="custom">Custom (OIDC or SAML)</SelectItem>
            </SelectContent>
          </Select>
          {PRESETS[v.preset].note && (
            <p className="text-xs text-muted-foreground bg-blue-500/5 border border-blue-500/20 rounded p-2">
              {PRESETS[v.preset].note}
            </p>
          )}
        </div>
      )}

      {/* Setup instructions (create only) */}
      {!isEdit && PRESETS[v.preset].setupSteps && (
        <SetupInstructions
          preset={v.preset}
          callbackUrl={callbackUrl}
          callbackUrlLabel={
            v.protocol === "oidc"
              ? "Redirect URI (Callback)"
              : "ACS URL (Assertion Consumer Service)"
          }
        />
      )}

      {/* Protocol (locked on edit) */}
      <div className="space-y-2">
        <Label>
          Protocol <RequiredMark />
        </Label>
        <Select
          value={v.protocol}
          onValueChange={(val) => update("protocol", val as "oidc" | "saml")}
          disabled={isEdit || (!isEdit && v.preset !== "custom")}
        >
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="oidc">OIDC (OAuth2 / OpenID Connect)</SelectItem>
            <SelectItem value="saml">SAML 2.0</SelectItem>
          </SelectContent>
        </Select>
        {isEdit && (
          <p className="text-xs text-muted-foreground">
            Protocol cannot be changed after creation.
          </p>
        )}
      </div>

      {/* Name + Slug */}
      <div className="grid grid-cols-2 gap-4">
        <FormField
          label="Name"
          required
          error={showError("name")}
        >
          <Input
            placeholder="Corporate Keycloak"
            value={v.name}
            onChange={(e) => update("name", e.target.value)}
            onBlur={() => markTouched("name")}
            aria-invalid={!!showError("name")}
          />
        </FormField>
        <FormField
          label="Slug"
          required
          error={showError("slug")}
          help={
            isEdit
              ? "Slug cannot be changed (would break registered redirect URIs)."
              : "Used in callback URLs."
          }
        >
          <Input
            placeholder="corporate-keycloak"
            value={v.slug}
            onChange={(e) => update("slug", e.target.value)}
            onBlur={() => markTouched("slug")}
            disabled={isEdit}
            aria-invalid={!!showError("slug")}
          />
        </FormField>
      </div>

      {/* Live callback URL preview — always visible on create, updates as slug changes */}
      {!isEdit && (
        <CallbackUrlPreview
          url={callbackUrl}
          protocol={v.protocol}
          invalid={!v.slug.trim() || !!errors.slug}
        />
      )}

      {/* OIDC fields */}
      {v.protocol === "oidc" ? (
        <>
          <FormField
            label="Issuer URL"
            required
            error={showError("issuerUrl")}
            help="Backend auto-discovers endpoints at /.well-known/openid-configuration"
          >
            <Input
              placeholder={PRESETS[v.preset].issuerHint || "https://idp.example.com"}
              value={v.issuerUrl}
              onChange={(e) => update("issuerUrl", e.target.value)}
              onBlur={() => markTouched("issuerUrl")}
              aria-invalid={!!showError("issuerUrl")}
            />
          </FormField>

          <FormField
            label="Client ID"
            required
            error={showError("clientId")}
          >
            <Input
              value={v.clientId}
              onChange={(e) => update("clientId", e.target.value)}
              onBlur={() => markTouched("clientId")}
              aria-invalid={!!showError("clientId")}
            />
          </FormField>

          <FormField
            label="Client Secret"
            help={
              isEdit && initialConfig?.oidc_secret_set
                ? "A secret is already set. Leave blank to keep the existing one, or enter a new value to replace it."
                : undefined
            }
          >
            <Input
              type="password"
              placeholder={
                isEdit && initialConfig?.oidc_secret_set ? "••••••••••" : ""
              }
              value={v.clientSecret}
              onChange={(e) => update("clientSecret", e.target.value)}
            />
          </FormField>

          <FormField label="Scopes">
            <Input
              placeholder="openid profile email"
              value={v.scopes}
              onChange={(e) => update("scopes", e.target.value)}
            />
          </FormField>
        </>
      ) : (
        <>
          <FormField
            label="IdP SSO URL"
            required
            error={showError("idpSsoUrl")}
          >
            <Input
              placeholder="https://idp.example.com/saml/sso"
              value={v.idpSsoUrl}
              onChange={(e) => update("idpSsoUrl", e.target.value)}
              onBlur={() => markTouched("idpSsoUrl")}
              aria-invalid={!!showError("idpSsoUrl")}
            />
          </FormField>

          <FormField label="IdP Entity ID">
            <Input
              value={v.idpEntityId}
              onChange={(e) => update("idpEntityId", e.target.value)}
            />
          </FormField>

          <FormField label="IdP Certificate (PEM)">
            <textarea
              className="w-full min-h-[80px] rounded-md border bg-background px-3 py-2 text-sm font-mono"
              placeholder="-----BEGIN CERTIFICATE-----"
              value={v.idpCertPem}
              onChange={(e) => update("idpCertPem", e.target.value)}
            />
          </FormField>
        </>
      )}

      {/* Role Mapping */}
      <div className="space-y-3 pt-2 border-t">
        <div>
          <Label className="text-sm font-semibold">Role Mapping</Label>
          <p className="text-xs text-muted-foreground mt-1">
            Map IdP groups/roles to internal roles. Users matching multiple
            rules get the highest privilege (admin &gt; user &gt; viewer).
            Users with no match get the <strong>default role</strong>.
          </p>
        </div>

        <FormField
          label="Claim / Attribute Name"
          required
          error={showError("roleClaimName")}
          help="Which OIDC claim or SAML attribute contains the groups (e.g. realm_access.roles for Keycloak)"
        >
          <Input
            placeholder="groups"
            value={v.roleClaimName}
            onChange={(e) => update("roleClaimName", e.target.value)}
            onBlur={() => markTouched("roleClaimName")}
            aria-invalid={!!showError("roleClaimName")}
          />
        </FormField>

        <div className="space-y-2">
          <Label className="text-xs flex items-center gap-2">
            <Badge variant="default" className="h-5">Admin</Badge>
            IdP groups that map to Admin
          </Label>
          <Input
            placeholder="Platform-Admins, Nexus-Admins"
            value={v.adminGroups}
            onChange={(e) => update("adminGroups", e.target.value)}
          />
        </div>

        <div className="space-y-2">
          <Label className="text-xs flex items-center gap-2">
            <Badge variant="secondary" className="h-5">User</Badge>
            IdP groups that map to User
          </Label>
          <Input
            placeholder="Engineers, Developers"
            value={v.userGroups}
            onChange={(e) => update("userGroups", e.target.value)}
          />
        </div>

        <div className="space-y-2">
          <Label className="text-xs flex items-center gap-2">
            <Badge variant="outline" className="h-5">Viewer</Badge>
            IdP groups that map to Viewer (read-only)
          </Label>
          <Input
            placeholder="Support, QA, Observers"
            value={v.viewerGroups}
            onChange={(e) => update("viewerGroups", e.target.value)}
          />
        </div>

        <p className="text-xs text-muted-foreground">
          Separate multiple groups with commas. Leave blank to disable a role mapping.
        </p>
      </div>

      {/* Fallback defaults */}
      <div className="space-y-3 pt-2 border-t">
        <div className="space-y-2">
          <Label>
            Default Role
            <span className="text-muted-foreground font-normal ml-1 text-xs">
              (used when no role mapping matches)
            </span>
          </Label>
          <Select
            value={v.defaultRole}
            onValueChange={(val) =>
              update("defaultRole", val as "admin" | "user" | "viewer")
            }
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="viewer">Viewer (read-only)</SelectItem>
              <SelectItem value="user">User (create resources)</SelectItem>
              <SelectItem value="admin">Admin (full access)</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="flex items-center justify-between rounded-lg border p-3">
          <div className="space-y-0.5">
            <Label className="text-sm">Just-in-Time Provisioning</Label>
            <p className="text-xs text-muted-foreground">
              Automatically create a local account on first SSO login
            </p>
          </div>
          <Switch
            checked={v.allowJit}
            onCheckedChange={(checked) => update("allowJit", checked)}
          />
        </div>
      </div>

      {/* Submit */}
      {submitAttempted && !isValid && (
        <p className="text-xs text-destructive">
          Please fix the highlighted fields before submitting.
        </p>
      )}
      <Button type="submit" className="w-full" disabled={submitting}>
        {submitting
          ? "Saving..."
          : submitLabel ?? (isEdit ? "Save Changes" : "Create Provider")}
      </Button>
    </form>
  )
}

// ─── Helpers ────────────────────────────────────────────────────────

function RequiredMark() {
  return <span className="text-destructive ml-0.5">*</span>
}

// ─── Setup instructions panel ──────────────────────────────────────

function SetupInstructions({
  preset,
  callbackUrl,
  callbackUrlLabel,
}: {
  preset: Preset
  callbackUrl: string
  callbackUrlLabel: string
}) {
  const cfg = PRESETS[preset]
  const [open, setOpen] = useState(true)

  if (!cfg.setupSteps) return null

  return (
    <div className="rounded-lg border border-violet-500/20 bg-violet-500/5 overflow-hidden">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className="w-full flex items-center justify-between gap-2 p-3 text-left hover:bg-violet-500/10 transition-colors"
      >
        <div className="flex items-center gap-2">
          <Info className="h-4 w-4 text-violet-600 dark:text-violet-400 shrink-0" />
          <span className="text-sm font-semibold">
            Set up {cfg.label} first
          </span>
        </div>
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <span>{open ? "Hide" : "Show"}</span>
        </div>
      </button>
      {open && (
        <div className="px-3 pb-3 space-y-3 border-t border-violet-500/20">
          <p className="text-xs text-muted-foreground pt-3">
            Complete these steps in your identity provider <strong>before</strong>{" "}
            saving this form. You&apos;ll need the redirect URI from below.
          </p>
          <ol className="space-y-1.5 text-xs list-decimal list-inside">
            {cfg.setupSteps.map((step, i) => (
              <li key={i} className="leading-relaxed">
                {step}
              </li>
            ))}
          </ol>
          {cfg.docsUrl && (
            <a
              href={cfg.docsUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1 text-xs text-violet-600 dark:text-violet-400 hover:underline"
            >
              Official documentation
              <ExternalLink className="h-3 w-3" />
            </a>
          )}
          <p className="text-xs text-muted-foreground pt-1">
            ↓ Copy the <strong>{callbackUrlLabel}</strong> shown below into
            your IdP&apos;s client configuration.
          </p>
        </div>
      )}
    </div>
  )
}

// ─── Live callback URL preview ──────────────────────────────────────

function CallbackUrlPreview({
  url,
  protocol,
  invalid,
}: {
  url: string
  protocol: "oidc" | "saml"
  invalid: boolean
}) {
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    if (invalid) return
    try {
      await navigator.clipboard.writeText(url)
      setCopied(true)
      toast.success("Callback URL copied to clipboard")
      setTimeout(() => setCopied(false), 2000)
    } catch {
      toast.error("Failed to copy — please select and copy manually")
    }
  }

  const label =
    protocol === "oidc"
      ? "Redirect URI"
      : "ACS URL (Assertion Consumer Service)"

  return (
    <div className="rounded-lg border bg-muted/40 p-3 space-y-1.5">
      <div className="flex items-center justify-between gap-2">
        <Label className="text-xs">
          {label} <span className="text-muted-foreground font-normal">(register this in your IdP)</span>
        </Label>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={handleCopy}
          disabled={invalid}
          className="h-6 px-2 text-xs"
        >
          {copied ? (
            <>
              <Check className="h-3 w-3 mr-1" />
              Copied
            </>
          ) : (
            <>
              <Copy className="h-3 w-3 mr-1" />
              Copy
            </>
          )}
        </Button>
      </div>
      <code
        className={`block text-xs font-mono break-all px-2 py-1.5 rounded ${
          invalid
            ? "bg-muted text-muted-foreground italic"
            : "bg-background border"
        }`}
      >
        {url}
      </code>
      {invalid && (
        <p className="text-xs text-muted-foreground">
          Enter a valid slug above to generate the callback URL.
        </p>
      )}
    </div>
  )
}

function FormField({
  label,
  required,
  error,
  help,
  children,
}: {
  label: string
  required?: boolean
  error?: string | false | undefined
  help?: string
  children: React.ReactNode
}) {
  return (
    <div className="space-y-2">
      <Label>
        {label}
        {required && <RequiredMark />}
      </Label>
      {children}
      {error ? (
        <p className="text-xs text-destructive">{error}</p>
      ) : help ? (
        <p className="text-xs text-muted-foreground">{help}</p>
      ) : null}
    </div>
  )
}
