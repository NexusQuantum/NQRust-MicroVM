"use client"

import { useState, useCallback } from "react"
import { DocsTopbar } from "@/components/docs/docs-topbar"
import { DocsSidebar } from "@/components/docs/docs-sidebar"
import { DocsSearch } from "@/components/docs/docs-search"
import { Sheet, SheetContent } from "@/components/ui/sheet"
import navIndexData from "@/content/api/_index.json"
import type { NavIndex } from "@/lib/docs/openapi-types"

const navIndex = navIndexData as NavIndex

export default function DocsLayout({ children }: { children: React.ReactNode }) {
  const [searchOpen, setSearchOpen] = useState(false)
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)

  const handleSearchOpen = useCallback(() => setSearchOpen(true), [])
  const handleMenuToggle = useCallback(
    () => setMobileMenuOpen((prev) => !prev),
    []
  )

  return (
    <div className="flex h-screen flex-col bg-background">
      <DocsTopbar
        onSearchOpen={handleSearchOpen}
        onMenuToggle={handleMenuToggle}
      />
      <div className="flex flex-1 overflow-hidden">
        {/* Desktop sidebar */}
        <aside className="hidden w-64 shrink-0 border-r border-border bg-card lg:block">
          <DocsSidebar navIndex={navIndex} className="h-full" />
        </aside>

        {/* Mobile sidebar */}
        <Sheet open={mobileMenuOpen} onOpenChange={setMobileMenuOpen}>
          <SheetContent side="left" className="w-72 p-0">
            <DocsSidebar navIndex={navIndex} className="h-full" />
          </SheetContent>
        </Sheet>

        {/* Main content */}
        <main className="flex-1 overflow-y-auto">{children}</main>
      </div>

      <DocsSearch
        navIndex={navIndex}
        open={searchOpen}
        onOpenChange={setSearchOpen}
      />
    </div>
  )
}
