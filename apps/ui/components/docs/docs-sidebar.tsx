"use client"

import { useState, useMemo } from "react"
import Link from "next/link"
import { usePathname } from "next/navigation"
import { ChevronRight } from "lucide-react"
import { cn } from "@/lib/utils"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import { MethodBadge } from "./method-badge"
import type { NavIndex } from "@/lib/docs/openapi-types"

export function DocsSidebar({
  navIndex,
  className,
}: {
  navIndex: NavIndex
  className?: string
}) {
  const pathname = usePathname()
  const [filter, setFilter] = useState("")
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({})

  const filteredTags = useMemo(() => {
    if (!filter) return navIndex.tags
    const lower = filter.toLowerCase()
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
      .filter((tag) => tag.endpoints.length > 0)
  }, [navIndex.tags, filter])

  const toggleTag = (slug: string) => {
    setCollapsed((prev) => ({ ...prev, [slug]: !prev[slug] }))
  }

  return (
    <div className={cn("flex h-full flex-col", className)}>
      <div className="p-3">
        <Input
          placeholder="Filter endpoints..."
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className="h-8 text-sm"
        />
      </div>
      <ScrollArea className="flex-1">
        <nav className="space-y-1 px-2 pb-4">
          {filteredTags.map((tag) => {
            const isCollapsed = collapsed[tag.slug] ?? false
            return (
              <div key={tag.slug}>
                <button
                  className="flex w-full items-center gap-1.5 rounded-md px-2 py-1.5 text-xs font-semibold uppercase tracking-wider text-muted-foreground hover:bg-muted/50"
                  onClick={() => toggleTag(tag.slug)}
                >
                  <ChevronRight
                    className={cn(
                      "h-3.5 w-3.5 transition-transform",
                      !isCollapsed && "rotate-90"
                    )}
                  />
                  {tag.name}
                  <span className="ml-auto text-[10px] font-normal">
                    {tag.endpoints.length}
                  </span>
                </button>
                {!isCollapsed && (
                  <div className="ml-2 space-y-0.5 py-0.5">
                    {tag.endpoints.map((ep) => {
                      const href = `/docs/${tag.slug}/${ep.slug}`
                      const isActive = pathname === href
                      return (
                        <Link
                          key={ep.slug}
                          href={href}
                          className={cn(
                            "flex items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors",
                            isActive
                              ? "bg-orange-500 text-white"
                              : "text-muted-foreground hover:bg-muted/50 hover:text-foreground"
                          )}
                        >
                          <MethodBadge
                            method={ep.method}
                            size="sm"
                            className={isActive ? "bg-white/20 text-white" : ""}
                          />
                          <span className="truncate font-mono text-xs">
                            {ep.path.replace("/v1/", "/")}
                          </span>
                        </Link>
                      )
                    })}
                  </div>
                )}
              </div>
            )
          })}
        </nav>
      </ScrollArea>
    </div>
  )
}
