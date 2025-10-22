import { apiGet, apiPost, apiPut, apiDelete } from "./client"
import type { Function, FunctionInvocation } from "@/lib/types"

export async function getFunctions(): Promise<Function[]> {
  return apiGet<Function[]>("/v1/functions")
}

export async function getFunction(id: string): Promise<Function> {
  return apiGet<Function>(`/v1/functions/${id}`)
}

export async function createFunction(data: any): Promise<Function> {
  return apiPost<Function>("/v1/functions", data)
}

export async function updateFunction(id: string, data: any): Promise<Function> {
  return apiPut<Function>(`/v1/functions/${id}`, data)
}

export async function deleteFunction(id: string): Promise<void> {
  return apiDelete(`/v1/functions/${id}`)
}

export async function invokeFunction(id: string, event: any): Promise<any> {
  return apiPost(`/v1/functions/${id}/invoke`, { event })
}

export async function getFunctionLogs(id: string): Promise<FunctionInvocation[]> {
  return apiGet<FunctionInvocation[]>(`/v1/functions/${id}/logs`)
}
