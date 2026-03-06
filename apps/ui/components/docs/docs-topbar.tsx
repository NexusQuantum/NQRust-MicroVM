"use client"

import Link from "next/link"
import Image from "next/image"
import { usePathname, useRouter } from "next/navigation"
import { Search, Menu, ArrowLeft } from "lucide-react"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"

const NAV_LINKS = [
  { label: "API Reference", href: "/docs" },
]

export function DocsTopbar({
  onSearchOpen,
  onMenuToggle,
}: {
  onSearchOpen: () => void
  onMenuToggle: () => void
}) {
  const pathname = usePathname()
  const router = useRouter()

  return (
    <header className="sticky top-0 z-30 flex h-14 items-center border-b border-border bg-card px-4">
      <Button
        variant="ghost"
        size="icon"
        className="mr-2 lg:hidden"
        onClick={onMenuToggle}
      >
        <Menu className="h-5 w-5" />
      </Button>

      <Button
        variant="ghost"
        size="icon"
        className="mr-2"
        onClick={() => router.back()}
        title="Go back"
      >
        <ArrowLeft className="h-5 w-5" />
      </Button>

      <Link href="/" className="flex items-center gap-2" prefetch={false}>
        <Image
          src="/nqr-logo-full.png"
          alt="NQR-MicroVM"
          width={120}
          height={32}
          className="h-8 w-auto"
          priority
        />
      </Link>

      <nav className="ml-6 hidden items-center gap-1 sm:flex">
        {NAV_LINKS.map((link) => (
          <Link
            key={link.href}
            href={link.href}
            className={cn(
              "rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
              pathname === link.href || (link.href !== "/docs" && pathname?.startsWith(link.href))
                ? "bg-orange-500/10 text-orange-600 dark:text-orange-400"
                : "text-muted-foreground hover:text-foreground"
            )}
          >
            {link.label}
          </Link>
        ))}
      </nav>

      <div className="ml-auto flex items-center gap-2">
        <Button
          variant="outline"
          size="sm"
          className="hidden gap-2 text-muted-foreground sm:flex"
          onClick={onSearchOpen}
        >
          <Search className="h-4 w-4" />
          <span className="text-xs">Search...</span>
          <kbd className="pointer-events-none ml-2 inline-flex h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground">
            <span className="text-xs">Ctrl</span>K
          </kbd>
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="sm:hidden"
          onClick={onSearchOpen}
        >
          <Search className="h-5 w-5" />
        </Button>
      </div>
    </header>
  )
}
