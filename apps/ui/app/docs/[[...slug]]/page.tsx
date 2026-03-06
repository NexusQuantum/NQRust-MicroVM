"use client"

import { use, useMemo } from "react"
import { notFound } from "next/navigation"
import Link from "next/link"
import { MethodBadge } from "@/components/docs/method-badge"
import { ApiEndpoint } from "@/components/docs/api-endpoint"
import { CopyButton } from "@/components/docs/copy-button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import navIndexData from "@/content/api/_index.json"
import type { NavIndex, NavTag } from "@/lib/docs/openapi-types"
import type { ParsedTag } from "@/lib/docs/openapi-types"
import { ArrowRight, Terminal, Key, Zap } from "lucide-react"

const navIndex = navIndexData as NavIndex

// Static map of tag JSON imports — using dynamic imports
const TAG_LOADERS: Record<string, () => Promise<{ default: ParsedTag }>> = {
  auth: () => import("@/content/api/auth.json"),
  containers: () => import("@/content/api/containers.json"),
  functions: () => import("@/content/api/functions.json"),
  hosts: () => import("@/content/api/hosts.json"),
  images: () => import("@/content/api/images.json"),
  logs: () => import("@/content/api/logs.json"),
  snapshots: () => import("@/content/api/snapshots.json"),
  templates: () => import("@/content/api/templates.json"),
  users: () => import("@/content/api/users.json"),
  "vm-configuration": () => import("@/content/api/vm-configuration.json"),
  "vm-devices": () => import("@/content/api/vm-devices.json"),
  vms: () => import("@/content/api/vms.json"),
}

const TAG_ICONS: Record<string, React.ReactNode> = {
  VMs: <Terminal className="h-5 w-5 text-orange-500" />,
  Auth: <Key className="h-5 w-5 text-orange-500" />,
  Functions: <Zap className="h-5 w-5 text-orange-500" />,
}

function DocsLandingPage() {
  const baseUrl = typeof window !== "undefined"
    ? `${window.location.protocol}//${window.location.hostname}:18080`
    : "http://localhost:18080"

  return (
    <div className="mx-auto max-w-4xl px-6 py-12">
      {/* Hero */}
      <div className="mb-12">
        <h1 className="text-4xl font-bold tracking-tight text-foreground">
          NQR-MicroVM API Reference
        </h1>
        <p className="mt-4 text-lg text-muted-foreground">
          Complete API documentation for managing virtual machines, containers,
          serverless functions, and more.
        </p>
      </div>

      {/* Base URL */}
      <div className="mb-10">
        <h2 className="mb-3 text-lg font-semibold text-foreground">Base URL</h2>
        <div className="flex items-center gap-2 rounded-lg border border-border bg-zinc-900 px-4 py-3">
          <code className="flex-1 font-mono text-sm text-zinc-100">{baseUrl}/v1</code>
          <CopyButton value={`${baseUrl}/v1`} className="text-zinc-400 hover:text-white" />
        </div>
      </div>

      {/* Authentication */}
      <div className="mb-10">
        <h2 className="mb-3 text-lg font-semibold text-foreground">Authentication</h2>
        <p className="mb-4 text-sm text-muted-foreground">
          Most endpoints require a Bearer token. Obtain one by calling the login endpoint:
        </p>
        <div className="rounded-lg border border-border bg-zinc-900 p-4">
          <div className="mb-2 flex items-center gap-2">
            <MethodBadge method="POST" size="sm" />
            <code className="font-mono text-sm text-zinc-100">/v1/auth/login</code>
          </div>
          <pre className="mt-3 overflow-x-auto text-xs text-zinc-300">
            {`curl -X POST \\
  -H "Content-Type: application/json" \\
  -d '{"username": "admin", "password": "your-password"}' \\
  "${baseUrl}/v1/auth/login"`}
          </pre>
          <p className="mt-3 text-xs text-zinc-400">
            The response includes a <code className="text-orange-400">token</code> field.
            Include it in subsequent requests as:
          </p>
          <pre className="mt-2 text-xs text-zinc-300">
            {`Authorization: Bearer <your-token>`}
          </pre>
        </div>
      </div>

      {/* Quick Start */}
      <div className="mb-10">
        <h2 className="mb-3 text-lg font-semibold text-foreground">Your First API Call</h2>
        <p className="mb-4 text-sm text-muted-foreground">
          After authenticating, list your virtual machines:
        </p>
        <div className="rounded-lg border border-border bg-zinc-900 p-4">
          <div className="mb-2 flex items-center gap-2">
            <MethodBadge method="GET" size="sm" />
            <code className="font-mono text-sm text-zinc-100">/v1/vms</code>
          </div>
          <pre className="mt-3 overflow-x-auto text-xs text-zinc-300">
            {`curl -H "Authorization: Bearer <your-token>" \\
  "${baseUrl}/v1/vms"`}
          </pre>
        </div>
      </div>

      {/* API sections */}
      <div>
        <h2 className="mb-4 text-lg font-semibold text-foreground">API Sections</h2>
        <div className="grid gap-3 sm:grid-cols-2">
          {navIndex.tags.map((tag) => (
            <Link key={tag.slug} href={`/docs/${tag.slug}`}>
              <Card className="h-full transition-colors hover:border-orange-500/50 hover:bg-muted/50">
                <CardHeader className="pb-2">
                  <CardTitle className="flex items-center gap-2 text-base">
                    {TAG_ICONS[tag.name] ?? <Terminal className="h-5 w-5 text-orange-500" />}
                    {tag.name}
                  </CardTitle>
                  <CardDescription className="text-xs">
                    {tag.description || `${tag.endpoints.length} endpoints`}
                  </CardDescription>
                </CardHeader>
                <CardContent className="pt-0">
                  <div className="flex items-center gap-1 text-xs text-muted-foreground">
                    <span>{tag.endpoints.length} endpoints</span>
                    <ArrowRight className="ml-auto h-4 w-4" />
                  </div>
                </CardContent>
              </Card>
            </Link>
          ))}
        </div>
      </div>
    </div>
  )
}

// Cache promises so `use()` gets a stable reference across renders
const tagPromiseCache = new Map<string, Promise<{ default: ParsedTag }>>()

function getTagPromise(tagSlug: string): Promise<{ default: ParsedTag }> | null {
  const loader = TAG_LOADERS[tagSlug]
  if (!loader) return null

  let promise = tagPromiseCache.get(tagSlug)
  if (!promise) {
    promise = loader()
    tagPromiseCache.set(tagSlug, promise)
  }
  return promise
}

function useTagData(tagSlug: string): ParsedTag | null {
  const promise = getTagPromise(tagSlug)
  if (!promise) return null

  const mod = use(promise)
  return mod.default
}

function TagOverview({ tag, navTag }: { tag: ParsedTag; navTag: NavTag }) {
  return (
    <div className="mx-auto max-w-4xl px-6 py-8">
      <h1 className="text-3xl font-bold tracking-tight text-foreground">{tag.name}</h1>
      {tag.description && (
        <p className="mt-2 text-muted-foreground">{tag.description}</p>
      )}
      <div className="mt-8 space-y-2">
        {navTag.endpoints.map((ep) => (
          <Link
            key={ep.slug}
            href={`/docs/${navTag.slug}/${ep.slug}`}
            className="flex items-center gap-3 rounded-lg border border-border p-4 transition-colors hover:border-orange-500/50 hover:bg-muted/50"
          >
            <MethodBadge method={ep.method} />
            <code className="font-mono text-sm font-medium text-foreground">
              {ep.path}
            </code>
            {ep.summary && (
              <span className="ml-2 truncate text-sm text-muted-foreground">
                {ep.summary}
              </span>
            )}
            <ArrowRight className="ml-auto h-4 w-4 shrink-0 text-muted-foreground" />
          </Link>
        ))}
      </div>
    </div>
  )
}

function EndpointView({
  tag,
  endpointSlug,
}: {
  tag: ParsedTag
  endpointSlug: string
}) {
  const endpoint = useMemo(
    () => tag.endpoints.find((e) => e.slug === endpointSlug),
    [tag, endpointSlug]
  )

  if (!endpoint) return notFound()

  return (
    <div className="mx-auto max-w-7xl px-6">
      <ApiEndpoint endpoint={endpoint} />
    </div>
  )
}

export default function DocsSlugPage({
  params,
}: {
  params: Promise<{ slug?: string[] }>
}) {
  const { slug } = use(params)

  // No slug → show landing page
  if (!slug || slug.length === 0) return <DocsLandingPage />

  const tagSlug = slug[0]
  const endpointSlug = slug[1]

  const navTag = navIndex.tags.find((t) => t.slug === tagSlug)
  if (!navTag) return notFound()

  const tag = useTagData(tagSlug)
  if (!tag) return notFound()

  if (!endpointSlug) {
    return <TagOverview tag={tag} navTag={navTag} />
  }

  return <EndpointView tag={tag} endpointSlug={endpointSlug} />
}
