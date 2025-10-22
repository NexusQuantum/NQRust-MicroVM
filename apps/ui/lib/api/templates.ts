import { apiGet, apiPost, apiPut, apiDelete } from "./client"
import type { Template } from "@/lib/types"

export async function getTemplates(): Promise<Template[]> {
  return apiGet<Template[]>("/v1/templates")
}

export async function getTemplate(id: string): Promise<Template> {
  return apiGet<Template>(`/v1/templates/${id}`)
}

export async function createTemplate(data: any): Promise<Template> {
  return apiPost<Template>("/v1/templates", data)
}

export async function updateTemplate(id: string, data: any): Promise<Template> {
  return apiPut<Template>(`/v1/templates/${id}`, data)
}

export async function deleteTemplate(id: string): Promise<void> {
  return apiDelete(`/v1/templates/${id}`)
}

export async function instantiateTemplate(id: string, data: any): Promise<any> {
  return apiPost(`/v1/templates/${id}/instantiate`, data)
}
