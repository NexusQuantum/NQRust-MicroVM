import { apiClient } from "./http"

export interface SsoProvider {
  slug: string
  name: string
  protocol: "oidc" | "saml"
  icon_hint?: string
  display_order: number
}

export interface SsoProviderConfig {
  id: string
  name: string
  slug: string
  protocol: "oidc" | "saml"
  enabled: boolean
  oidc_issuer_url?: string
  oidc_client_id?: string
  oidc_secret_set: boolean
  oidc_scopes?: string
  saml_idp_entity_id?: string
  saml_idp_sso_url?: string
  saml_sp_entity_id?: string
  role_mapping: Record<string, string[]>
  role_claim_name?: string
  default_role: "admin" | "user" | "viewer"
  allow_jit_provisioning: boolean
  username_claim?: string
  email_claim?: string
  display_name_claim?: string
  icon_hint?: string
  display_order: number
  created_at: string
  updated_at: string
}

export interface CreateSsoProviderRequest {
  name: string
  slug: string
  protocol: "oidc" | "saml"
  oidc_issuer_url?: string
  oidc_client_id?: string
  oidc_client_secret?: string
  oidc_scopes?: string
  saml_idp_metadata_xml?: string
  saml_idp_sso_url?: string
  saml_idp_entity_id?: string
  saml_idp_certificate_pem?: string
  saml_sp_entity_id?: string
  role_mapping?: Record<string, string[]>
  role_claim_name?: string
  default_role?: "admin" | "user" | "viewer"
  allow_jit_provisioning?: boolean
  icon_hint?: string
  display_order?: number
}

export interface SsoTestResult {
  success: boolean
  message?: string
  error?: string
}

export const ssoApi = {
  /** List enabled SSO providers (public, no auth required) */
  async getProviders(): Promise<SsoProvider[]> {
    const res = await apiClient.get<{ providers: SsoProvider[] }>("/sso/providers")
    return res.providers
  },

  /** Admin: list all SSO providers */
  async listAll(): Promise<SsoProviderConfig[]> {
    const res = await apiClient.get<{ items: SsoProviderConfig[] }>("/admin/sso/providers")
    return res.items
  },

  /** Admin: create SSO provider */
  async create(req: CreateSsoProviderRequest): Promise<SsoProviderConfig> {
    return apiClient.post<SsoProviderConfig>("/admin/sso/providers", req)
  },

  /** Admin: get SSO provider */
  async get(id: string): Promise<SsoProviderConfig> {
    return apiClient.get<SsoProviderConfig>(`/admin/sso/providers/${id}`)
  },

  /** Admin: update SSO provider */
  async update(id: string, req: Partial<CreateSsoProviderRequest> & { enabled?: boolean }): Promise<SsoProviderConfig> {
    return apiClient.patch<SsoProviderConfig>(`/admin/sso/providers/${id}`, req)
  },

  /** Admin: delete SSO provider */
  async delete(id: string): Promise<void> {
    await apiClient.delete(`/admin/sso/providers/${id}`)
  },

  /** Admin: test SSO provider connectivity */
  async test(id: string): Promise<SsoTestResult> {
    return apiClient.post<SsoTestResult>(`/admin/sso/providers/${id}/test`, {})
  },
}
