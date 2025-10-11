import { NextRequest, NextResponse } from "next/server"

const MANAGER_BASE = process.env.MANAGER_BASE || "http://127.0.0.1:18080"

export const dynamic = "force-dynamic"

async function proxy(req: NextRequest, { params }: { params: { path: string[] } }) {
  const upstream = `${MANAGER_BASE}/${params.path.join("/")}`

  const init: RequestInit = {
    method: req.method,
    headers: new Headers(req.headers),
    body: req.method === "GET" || req.method === "HEAD" ? undefined : await req.text(),
    redirect: "manual",
  }

  const resp = await fetch(upstream, init)
  const headers = new Headers(resp.headers)
  headers.set("access-control-allow-origin", "*")
  return new NextResponse(resp.body, { status: resp.status, headers })
}

export { proxy as GET, proxy as POST, proxy as PUT, proxy as PATCH, proxy as DELETE }


