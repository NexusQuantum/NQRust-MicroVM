"use client"

import { useEffect, useMemo, useState, useCallback } from "react"
import { useRouter } from "next/navigation"
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command"
import { MethodBadge } from "./method-badge"
import type { NavIndex } from "@/lib/docs/openapi-types"

export function DocsSearch({
  navIndex,
  open,
  onOpenChange,
}: {
  navIndex: NavIndex
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const router = useRouter()
  const [query, setQuery] = useState("")

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault()
        onOpenChange(!open)
      }
    }
    document.addEventListener("keydown", handleKeyDown)
    return () => document.removeEventListener("keydown", handleKeyDown)
  }, [open, onOpenChange])

  const filteredTags = useMemo(() => {
    if (!query) return navIndex.tags
    const lower = query.toLowerCase()
    return navIndex.tags
      .map((tag) => ({
        ...tag,
        endpoints: tag.endpoints.filter(
          (e) =>
            e.path.toLowerCase().includes(lower) ||
            e.method.toLowerCase().includes(lower) ||
            e.summary?.toLowerCase().includes(lower) ||
            tag.name.toLowerCase().includes(lower)
        ),
      }))
      .filter((t) => t.endpoints.length > 0)
  }, [navIndex.tags, query])

  const handleSelect = useCallback(
    (tagSlug: string, epSlug: string) => {
      router.push(`/docs/${tagSlug}/${epSlug}`)
      onOpenChange(false)
      setQuery("")
    },
    [router, onOpenChange]
  )

  return (
    <CommandDialog open={open} onOpenChange={onOpenChange}>
      <CommandInput
        placeholder="Search endpoints..."
        value={query}
        onValueChange={setQuery}
      />
      <CommandList>
        <CommandEmpty>No endpoints found.</CommandEmpty>
        {filteredTags.map((tag) => (
          <CommandGroup key={tag.slug} heading={tag.name}>
            {tag.endpoints.map((ep) => (
              <CommandItem
                key={`${tag.slug}-${ep.slug}`}
                value={`${ep.method} ${ep.path} ${ep.summary ?? ""} ${tag.name}`}
                onSelect={() => handleSelect(tag.slug, ep.slug)}
                className="flex items-center gap-2"
              >
                <MethodBadge method={ep.method} size="sm" />
                <span className="font-mono text-sm">{ep.path}</span>
                {ep.summary && (
                  <span className="ml-auto truncate text-xs text-muted-foreground">
                    {ep.summary}
                  </span>
                )}
              </CommandItem>
            ))}
          </CommandGroup>
        ))}
      </CommandList>
    </CommandDialog>
  )
}
