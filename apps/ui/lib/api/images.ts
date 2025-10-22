import { apiGet, apiPost, apiDelete } from "./client"
import type { Image } from "@/lib/types"

export async function getImages(): Promise<Image[]> {
  return apiGet<Image[]>("/v1/images")
}

export async function getImage(id: string): Promise<Image> {
  return apiGet<Image>(`/v1/images/${id}`)
}

export async function createImage(data: any): Promise<Image> {
  return apiPost<Image>("/v1/images", data)
}

export async function deleteImage(id: string): Promise<void> {
  return apiDelete(`/v1/images/${id}`)
}
