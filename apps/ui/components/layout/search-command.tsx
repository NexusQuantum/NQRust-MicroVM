"use client"

import * as React from "react"
import { useRouter } from "next/navigation"
import { cn } from "@/lib/utils"
import {
  Search,
  X,
  ArrowRight,
  ChevronRight,
  ChevronDown,
  Star,
  Loader2,
  Server,
  Zap,
  Container,
  FileCode,
  Database,
  ServerCog,
  Network,
  HardDrive,
  Plus,
  CodeXml,
  List,
  Upload,
} from "lucide-react"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
  useVMs,
  useFunctions,
  useContainers,
  useTemplates,
  useRegistryImages,
  useHosts,
  useNetworks,
  useVolumes,
} from "@/lib/queries"

interface Service {
  name: string
  href: string
}

type IconType = React.ComponentType<{ className?: string }>

interface Feature {
  name: string
  href: string
  service: string
  expandable?: boolean
  dataKey?: string
  icon?: IconType
}

// Services (text only, from Virtual Machines to Volumes)
const SERVICES: Service[] = [
  { name: "Virtual Machines", href: "/vms" },
  { name: "Functions", href: "/functions" },
  { name: "Containers", href: "/containers" },
  { name: "Templates", href: "/templates" },
  { name: "Registry", href: "/registry" },
  { name: "Hosts", href: "/hosts" },
  { name: "Networks", href: "/networks" },
  { name: "Volumes", href: "/volumes" },
]

// Features/Actions grouped by service
const FEATURE_GROUPS: { service: string; features: Feature[] }[] = [
  {
    service: "Virtual Machines",
    features: [
      { name: "View VMs", href: "/vms", service: "Virtual Machines", expandable: true, dataKey: "vms", icon: List },
      { name: "Create VM", href: "/vms/create", service: "Virtual Machines", icon: Plus },
    ],
  },
  {
    service: "Functions",
    features: [
      { name: "View Functions", href: "/functions", service: "Functions", expandable: true, dataKey: "functions", icon: List },
      { name: "Create Function", href: "/functions/new", service: "Functions", icon: Plus },
      { name: "Playground", href: "/functions/playground", service: "Functions", icon: CodeXml },
    ],
  },
  {
    service: "Containers",
    features: [
      { name: "View Containers", href: "/containers", service: "Containers", expandable: true, dataKey: "containers", icon: List },
      { name: "Create Container", href: "/containers/new", service: "Containers", icon: Plus },
    ],
  },
  {
    service: "Templates",
    features: [
      { name: "View Templates", href: "/templates", service: "Templates", expandable: true, dataKey: "templates", icon: List },
      { name: "Create Template", href: "/templates/new", service: "Templates", icon: Plus },
    ],
  },
  {
    service: "Registry",
    features: [
      { name: "View Images", href: "/registry", service: "Registry", expandable: true, dataKey: "images", icon: List },
      { name: "Import Image", href: "/registry/import", service: "Registry", icon: Upload },
    ],
  },
  {
    service: "Hosts",
    features: [
      { name: "View Hosts", href: "/hosts", service: "Hosts", expandable: true, dataKey: "hosts", icon: List },
    ],
  },
  {
    service: "Networks",
    features: [
      { name: "View Networks", href: "/networks", service: "Networks", expandable: true, dataKey: "networks", icon: List },
    ],
  },
  {
    service: "Volumes",
    features: [
      { name: "View Volumes", href: "/volumes", service: "Volumes", expandable: true, dataKey: "volumes", icon: List },
    ],
  },
]

// Data item display configuration
const DATA_CONFIG: Record<string, {
  icon: React.ComponentType<{ className?: string }>
  getLabel: (item: Record<string, unknown>) => string
  getHref: (item: Record<string, unknown>) => string
  getStatus?: (item: Record<string, unknown>) => { label: string; color: string }
}> = {
  vms: {
    icon: Server,
    getLabel: (item) => (item.name as string) || (item.id as string),
    getHref: (item) => `/vms/${item.id}`,
    getStatus: (item) => {
      const state = item.state as string
      const colors: Record<string, string> = {
        running: "text-green-500",
        stopped: "text-gray-500",
        paused: "text-yellow-500",
        failed: "text-red-500",
      }
      return { label: state, color: colors[state] || "text-gray-500" }
    },
  },
  functions: {
    icon: Zap,
    getLabel: (item) => (item.name as string) || (item.id as string),
    getHref: (item) => `/functions/${item.id}`,
    getStatus: (item) => {
      const runtime = item.runtime as string
      return { label: runtime, color: "text-blue-500" }
    },
  },
  containers: {
    icon: Container,
    getLabel: (item) => (item.name as string) || (item.id as string),
    getHref: (item) => `/containers/${item.id}`,
    getStatus: (item) => {
      const state = item.state as string
      const colors: Record<string, string> = {
        running: "text-green-500",
        stopped: "text-gray-500",
        pending: "text-yellow-500",
        failed: "text-red-500",
      }
      return { label: state, color: colors[state] || "text-gray-500" }
    },
  },
  templates: {
    icon: FileCode,
    getLabel: (item) => (item.name as string) || (item.id as string),
    getHref: (item) => `/templates/${item.id}`,
  },
  images: {
    icon: Database,
    getLabel: (item) => (item.name as string) || (item.id as string),
    getHref: (item) => `/registry/${item.id}`,
    getStatus: (item) => {
      const kind = item.kind as string
      return { label: kind, color: "text-purple-500" }
    },
  },
  hosts: {
    icon: ServerCog,
    getLabel: (item) => (item.name as string) || (item.id as string),
    getHref: (item) => `/hosts?id=${item.id}`,
    getStatus: (item) => {
      const status = item.status as string
      const colors: Record<string, string> = {
        online: "text-green-500",
        offline: "text-red-500",
      }
      return { label: status, color: colors[status] || "text-gray-500" }
    },
  },
  networks: {
    icon: Network,
    getLabel: (item) => (item.name as string) || (item.id as string),
    getHref: (item) => `/networks?id=${item.id}`,
  },
  volumes: {
    icon: HardDrive,
    getLabel: (item) => (item.name as string) || (item.id as string),
    getHref: (item) => `/volumes?id=${item.id}`,
    getStatus: (item) => {
      const format = item.format as string
      return { label: format, color: "text-cyan-500" }
    },
  },
}

// Expandable Feature Item Component
function FeatureItem({
  feature,
  onSelect,
  data,
  isLoading,
}: {
  feature: Feature
  onSelect: (href: string) => void
  data?: Record<string, unknown>[]
  isLoading?: boolean
}) {
  const [expanded, setExpanded] = React.useState(false)
  const config = feature.dataKey ? DATA_CONFIG[feature.dataKey] : null

  const handleExpandClick = (e: React.MouseEvent) => {
    e.stopPropagation()
    if (feature.expandable) {
      setExpanded(!expanded)
    }
  }

  const displayData = data?.slice(0, 3) || []
  const hasMore = (data?.length || 0) > 3

  const FeatureIcon = feature.icon

  return (
    <div className="divide-y divide-border/30">
      <button
        onClick={() => onSelect(feature.href)}
        className="flex w-full items-center justify-between px-4 py-3 text-left transition-all hover:bg-orange-500/5 group"
      >
        <div className="flex items-center gap-3">
          {feature.expandable ? (
            <div
              onClick={handleExpandClick}
              className="p-0.5 -m-0.5 rounded hover:bg-orange-500/10 cursor-pointer"
            >
              {expanded ? (
                <ChevronDown className="h-4 w-4 text-orange-500 transition-colors" />
              ) : (
                <ChevronRight className="h-4 w-4 text-muted-foreground/50 group-hover:text-orange-500 transition-colors" />
              )}
            </div>
          ) : FeatureIcon ? (
            <FeatureIcon className="h-4 w-4 text-muted-foreground/50 group-hover:text-orange-500 transition-colors" />
          ) : (
            <ChevronRight className="h-4 w-4 text-muted-foreground/50 group-hover:text-orange-500 transition-colors" />
          )}
          <span className="text-sm text-foreground group-hover:text-orange-500 transition-colors">
            {feature.name}
          </span>
        </div>
        <Star className="h-4 w-4 text-muted-foreground/30 opacity-0 group-hover:opacity-100 hover:text-yellow-500 transition-all" />
      </button>

      {/* Expanded Data Preview */}
      {expanded && feature.expandable && config && (
        <div className="bg-muted/20 px-4 py-2">
          {isLoading ? (
            <div className="flex items-center justify-center py-3">
              <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
              <span className="ml-2 text-xs text-muted-foreground">Loading...</span>
            </div>
          ) : displayData.length > 0 ? (
            <div className="space-y-1">
              {displayData.map((item, idx) => {
                const Icon = config.icon
                const status = config.getStatus?.(item)
                return (
                  <button
                    key={idx}
                    onClick={(e) => {
                      e.stopPropagation()
                      onSelect(config.getHref(item))
                    }}
                    className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left hover:bg-orange-500/10 transition-colors group/item"
                  >
                    <Icon className="h-3.5 w-3.5 text-muted-foreground/70" />
                    <span className="flex-1 truncate text-xs text-foreground/80 group-hover/item:text-orange-500">
                      {config.getLabel(item)}
                    </span>
                    {status && (
                      <span className={cn("text-[10px] capitalize", status.color)}>
                        {status.label}
                      </span>
                    )}
                  </button>
                )
              })}
              {hasMore && (
                <button
                  onClick={(e) => {
                    e.stopPropagation()
                    onSelect(feature.href)
                  }}
                  className="flex w-full items-center gap-2 px-2 py-1.5 text-xs text-orange-500 hover:text-orange-600 hover:underline"
                >
                  <ArrowRight className="h-3 w-3" />
                  Show all {data?.length} items
                </button>
              )}
            </div>
          ) : (
            <div className="py-2 text-center text-xs text-muted-foreground">
              No data available
            </div>
          )}
        </div>
      )}
    </div>
  )
}

export function SearchCommand() {
  const router = useRouter()
  const [open, setOpen] = React.useState(false)
  const [query, setQuery] = React.useState("")
  const [selectedService, setSelectedService] = React.useState<string | null>(null)
  const inputRef = React.useRef<HTMLInputElement>(null)
  const containerRef = React.useRef<HTMLDivElement>(null)

  // Fetch data for all expandable features
  const { data: vmsData, isLoading: vmsLoading } = useVMs(false, 0)
  const { data: functionsData, isLoading: functionsLoading } = useFunctions(0)
  const { data: containersData, isLoading: containersLoading } = useContainers(0)
  const { data: templatesData, isLoading: templatesLoading } = useTemplates()
  const { data: imagesData, isLoading: imagesLoading } = useRegistryImages()
  const { data: hostsData, isLoading: hostsLoading } = useHosts()
  const { data: networksData, isLoading: networksLoading } = useNetworks()
  const { data: volumesData, isLoading: volumesLoading } = useVolumes()

  // Map data by key
  const dataMap: Record<string, { data?: Record<string, unknown>[]; isLoading: boolean }> = {
    vms: { data: vmsData as Record<string, unknown>[], isLoading: vmsLoading },
    functions: { data: functionsData as Record<string, unknown>[], isLoading: functionsLoading },
    containers: { data: containersData as Record<string, unknown>[], isLoading: containersLoading },
    templates: { data: templatesData as Record<string, unknown>[], isLoading: templatesLoading },
    images: { data: imagesData as Record<string, unknown>[], isLoading: imagesLoading },
    hosts: { data: hostsData as Record<string, unknown>[], isLoading: hostsLoading },
    networks: { data: networksData as Record<string, unknown>[], isLoading: networksLoading },
    volumes: { data: volumesData as Record<string, unknown>[], isLoading: volumesLoading },
  }

  // Filter services based on query
  const filteredServices = React.useMemo(() => {
    if (!query) return SERVICES
    const lowerQuery = query.toLowerCase()
    return SERVICES.filter((s) => s.name.toLowerCase().includes(lowerQuery))
  }, [query])

  // Filter feature groups based on query and selected service
  const filteredFeatureGroups = React.useMemo(() => {
    let groups = FEATURE_GROUPS

    // Filter by selected service
    if (selectedService) {
      groups = groups.filter((g) => g.service === selectedService)
    }

    // Filter by search query
    if (query) {
      const lowerQuery = query.toLowerCase()
      groups = groups
        .map((group) => ({
          ...group,
          features: group.features.filter(
            (f) =>
              f.name.toLowerCase().includes(lowerQuery) ||
              f.service.toLowerCase().includes(lowerQuery)
          ),
        }))
        .filter((group) => group.features.length > 0)
    }

    return groups
  }, [query, selectedService])

  // Handle keyboard shortcut (Ctrl+K / Cmd+K)
  React.useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault()
        setOpen((prev) => !prev)
      }
      if (e.key === "Escape") {
        setOpen(false)
      }
    }

    document.addEventListener("keydown", handleKeyDown)
    return () => document.removeEventListener("keydown", handleKeyDown)
  }, [])

  // Focus input when opened
  React.useEffect(() => {
    if (open && inputRef.current) {
      inputRef.current.focus()
    }
  }, [open])

  // Handle click outside
  React.useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false)
      }
    }

    if (open) {
      document.addEventListener("mousedown", handleClickOutside)
    }
    return () => document.removeEventListener("mousedown", handleClickOutside)
  }, [open])

  const handleSelect = (href: string) => {
    router.push(href)
    setOpen(false)
    setQuery("")
    setSelectedService(null)
  }

  const handleServiceClick = (serviceName: string) => {
    setSelectedService(serviceName === selectedService ? null : serviceName)
  }

  return (
    <div className="relative w-96 max-w-full" ref={containerRef}>
      {/* Search Input */}
      <div className="relative">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          ref={inputRef}
          type="text"
          placeholder="Search resources... (Ctrl+K)"
          className="pl-9 pr-9 shadow-none"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onFocus={() => setOpen(true)}
        />
        {(query || open) && (
          <Button
            variant="ghost"
            size="icon"
            className="absolute right-1 top-1/2 h-7 w-7 -translate-y-1/2"
            onClick={() => {
              setQuery("")
              setOpen(false)
              setSelectedService(null)
            }}
          >
            <X className="h-4 w-4" />
          </Button>
        )}
      </div>

      {/* Search Popup */}
      {open && (
        <div className="absolute left-0 top-full z-50 mt-2 w-[700px] overflow-hidden rounded-lg border border-border bg-popover shadow-xl">
          <div className="flex min-h-[400px]">
            {/* Left Side - Services (Text Only) */}
            <div className="w-[200px] border-r border-border bg-muted/30">
              <ScrollArea className="h-[400px]">
                <div className="p-2">
                  <div className="px-3 py-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                    Services
                  </div>
                  {filteredServices.map((service) => {
                    const isSelected = selectedService === service.name

                    return (
                      <button
                        key={service.name}
                        onClick={() => handleServiceClick(service.name)}
                        className={cn(
                          "flex w-full items-center rounded-md px-3 py-2 text-left text-sm transition-colors",
                          isSelected
                            ? "bg-orange-500 text-white"
                            : "hover:bg-muted text-foreground"
                        )}
                      >
                        <span className="truncate">{service.name}</span>
                      </button>
                    )
                  })}
                </div>
              </ScrollArea>
            </div>

            {/* Right Side - Features grouped by headers */}
            <div className="flex-1 bg-background/50">
              <ScrollArea className="h-[400px]">
                <div className="p-4">
                  {filteredFeatureGroups.length > 0 ? (
                    <div className="space-y-5">
                      {filteredFeatureGroups.map((group) => (
                        <div key={group.service} className="rounded-lg border border-border/50 bg-card/50 overflow-hidden">
                          {/* Service Header */}
                          <div className="px-4 py-2.5 bg-muted/30 border-b border-border/50">
                            <h3 className="text-sm font-semibold text-orange-500">
                              {group.service}
                            </h3>
                          </div>
                          {/* Feature Items */}
                          <div>
                            {group.features.map((feature) => (
                              <FeatureItem
                                key={feature.href}
                                feature={feature}
                                onSelect={handleSelect}
                                data={feature.dataKey ? dataMap[feature.dataKey]?.data : undefined}
                                isLoading={feature.dataKey ? dataMap[feature.dataKey]?.isLoading : false}
                              />
                            ))}
                          </div>
                        </div>
                      ))}
                    </div>
                  ) : (
                    /* Empty state */
                    <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
                      <div className="rounded-full bg-muted/50 p-4 mb-4">
                        <Search className="h-8 w-8 opacity-50" />
                      </div>
                      <p className="text-sm font-medium">No results found</p>
                      <p className="text-xs mt-1 text-muted-foreground/70">
                        Try searching for &quot;{query}&quot; in services or features
                      </p>
                    </div>
                  )}

                  {/* Quick tips */}
                  {!query && !selectedService && (
                    <div className="mt-5 rounded-lg border border-border/50 bg-card/30 p-4">
                      <h3 className="pb-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                        Quick Tips
                      </h3>
                      <div className="space-y-2.5 text-xs text-muted-foreground">
                        <div className="flex items-center gap-3">
                          <kbd className="px-2 py-1 rounded-md bg-muted border border-border/50 text-[10px] font-mono font-medium">
                            Ctrl+K
                          </kbd>
                          <span>Open search</span>
                        </div>
                        <div className="flex items-center gap-3">
                          <kbd className="px-2 py-1 rounded-md bg-muted border border-border/50 text-[10px] font-mono font-medium">
                            Esc
                          </kbd>
                          <span>Close search</span>
                        </div>
                        <div className="flex items-center gap-3">
                          <ChevronRight className="h-3.5 w-3.5 text-orange-500" />
                          <span>Click arrow to preview data</span>
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              </ScrollArea>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
