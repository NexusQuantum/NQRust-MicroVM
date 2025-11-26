"use client"

import { useEffect } from "react"
import { useSearchParams, useRouter } from "next/navigation"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Plus } from "lucide-react"
import { TemplateList } from "@/components/templates/template-list"
import Link from "next/link"
import { useTemplates } from "@/lib/queries"
import { Skeleton } from "@/components/ui/skeleton"
import { AlertCircle } from "lucide-react"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { toast } from "sonner"

const TemplateFlowDiagram = () => (
  <svg width="320" height="200" viewBox="0 0 320 200" fill="none" xmlns="http://www.w3.org/2000/svg" className="drop-shadow-lg">
    <style>{`
      .template-text { fill: #ea580c; }
      .dark .template-text { fill: #fb923c; }
      .template-bg { fill: #fff7ed; }
      .dark .template-bg { fill: rgba(154, 52, 18, 0.3); }
      .template-vm-bg { fill: #fed7aa; }
      .dark .template-vm-bg { fill: rgba(154, 52, 18, 0.4); }
    `}</style>
    {/* Template Blueprint (Left) */}
    <rect x="15" y="60" width="100" height="85" rx="10" className="template-bg" stroke="#ea580c" strokeWidth="2.5" strokeDasharray="6,4" />
    <text x="65" y="80" textAnchor="middle" className="template-text" fontWeight="700" fontSize="14">Template</text>
    <line x1="30" y1="90" x2="100" y2="90" stroke="#fb923c" strokeWidth="1" opacity="0.3" />
    <text x="65" y="105" textAnchor="middle" className="template-text" fontSize="10.5">CPU: 2 cores</text>
    <text x="65" y="120" textAnchor="middle" className="template-text" fontSize="10.5">RAM: 4GB</text>
    <text x="65" y="135" textAnchor="middle" className="template-text" fontSize="10.5">Disk: 20GB</text>

    {/* Blueprint icon dots */}
    {/* <circle cx="50" cy="72" r="2.5" fill="#ea580c" opacity="0.5"/>
    <circle cx="65" cy="72" r="2.5" fill="#ea580c" opacity="0.5"/>
    <circle cx="80" cy="72" r="2.5" fill="#ea580c" opacity="0.5"/> */}

    {/* Deploy Arrow - Smooth curve */}
    <defs>
      <linearGradient id="arrowGradient" x1="0%" y1="0%" x2="100%" y2="0%">
        <stop offset="0%" stopColor="#ea580c" stopOpacity="0.8" />
        <stop offset="100%" stopColor="#fb923c" stopOpacity="0.9" />
      </linearGradient>
    </defs>

    <path d="M 115 102 L 145 102" stroke="url(#arrowGradient)" strokeWidth="2.5" fill="none" />
    <polygon points="148,102 141,97 141,107" fill="#fb923c" />
    <text x="140" y="94" textAnchor="middle" className="template-text" fontSize="10" fontWeight="700">Deploy</text>

    {/* Vertical line from arrow point */}
    <line x1="115" y1="102" x2="165" y2="102" stroke="#ea580c" strokeWidth="2" opacity="0.4" />

    {/* Connection hub/distributor */}
    <circle cx="165" cy="102" r="4" fill="#ea580c" opacity="0.8" />

    {/* Smooth curved lines to VMs */}
    <path d="M 165 102 Q 175 75, 190 50" stroke="#ea580c" strokeWidth="2" opacity="0.5" fill="none" strokeDasharray="3,2" />
    <path d="M 165 102 L 190 102" stroke="#ea580c" strokeWidth="2" opacity="0.5" fill="none" strokeDasharray="3,2" />
    <path d="M 165 102 Q 175 130, 190 158" stroke="#ea580c" strokeWidth="2" opacity="0.5" fill="none" strokeDasharray="3,2" />

    {/* Deployed VMs - with shadow effect */}
    {/* VM 1 */}
    <rect x="190" y="20" width="70" height="60" rx="8" className="template-vm-bg" stroke="#ea580c" strokeWidth="2.5" />
    <rect x="190" y="20" width="70" height="60" rx="8" fill="url(#vmGradient)" opacity="0.1" />
    <text x="225" y="46" textAnchor="middle" className="template-text" fontWeight="700" fontSize="12">VM 1</text>
    <text x="225" y="66" textAnchor="middle" fill="#16a34a" fontWeight="600" fontSize="9.5">● Running</text>

    {/* VM 2 */}
    <rect x="190" y="90" width="70" height="60" rx="8" className="template-vm-bg" stroke="#ea580c" strokeWidth="2.5" />
    <rect x="190" y="90" width="70" height="60" rx="8" fill="url(#vmGradient)" opacity="0.1" />
    <text x="225" y="116" textAnchor="middle" className="template-text" fontWeight="700" fontSize="12">VM 2</text>
    <text x="225" y="136" textAnchor="middle" fill="#16a34a" fontWeight="600" fontSize="9.5">● Running</text>

    {/* VM 3 */}
    <rect x="190" y="160" width="70" height="35" rx="8" className="template-vm-bg" stroke="#ea580c" strokeWidth="2.5" />
    <rect x="190" y="160" width="70" height="35" rx="8" fill="url(#vmGradient)" opacity="0.1" />
    <text x="225" y="181" textAnchor="middle" className="template-text" fontWeight="700" fontSize="12">VM 3</text>

    {/* Gradient for VMs */}
    <defs>
      <linearGradient id="vmGradient" x1="0%" y1="0%" x2="0%" y2="100%">
        <stop offset="0%" stopColor="#fb923c" stopOpacity="0.2" />
        <stop offset="100%" stopColor="#ea580c" stopOpacity="0" />
      </linearGradient>
    </defs>

    {/* Label with icon */}
    {/* <text x="275" y="100" textAnchor="start" fill="#9a3412" fontSize="11" fontWeight="700">Quick</text>
    <text x="275" y="114" textAnchor="start" fill="#9a3412" fontSize="11" fontWeight="700">Deploy</text>
    <circle cx="300" cy="107" r="8" fill="#ea580c" opacity="0.15"/>
    <text x="300" y="111" textAnchor="middle" fill="#ea580c" fontSize="16" fontWeight="700">⚡</text> */}
  </svg>
)

export default function TemplatesPage() {
  const searchParams = useSearchParams()
  const router = useRouter()
  const { data: templates = [], isLoading, error } = useTemplates()

  useEffect(() => {
    const action = searchParams.get("action")

    if (action === "updated") {
      toast.success("Template Updated", {
        description: "Template has been updated successfully",
      })
      // Remove query param
      router.replace("/templates")
    } else if (action === "deleted") {
      toast.success("Template Deleted", {
        description: "Template has been deleted successfully",
      })
      // Remove query param
      router.replace("/templates")
    }
  }, [searchParams, router])

  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-orange-50 to-orange-100/50 dark:from-orange-950/30 dark:to-orange-900/20 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">VM Templates</h1>
            <p className="mt-2 text-muted-foreground">
              Save and deploy VM configurations as templates. Quickly spin up new instances with pre-configured
              settings.
            </p>
            <Button asChild className="mt-4">
              <Link href="/templates/new">
                <Plus className="mr-2 h-4 w-4" />
                Create Template
              </Link>
            </Button>
          </div>
          <div className="hidden lg:block">
            <TemplateFlowDiagram />
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-orange-400/30 to-orange-600/30 dark:from-orange-500/20 dark:to-orange-600/10 blur-3xl" />
      </div>

      <Card>
        <CardHeader>
          <CardTitle>All Templates</CardTitle>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-4">
              {[...Array(3)].map((_, i) => (
                <div key={i} className="p-6 border rounded-lg space-y-4">
                  <div className="flex items-start justify-between">
                    <div className="space-y-2">
                      <Skeleton className="h-6 w-48" />
                      <Skeleton className="h-4 w-64" />
                    </div>
                    <Skeleton className="h-8 w-20" />
                  </div>
                  <div className="grid grid-cols-2 gap-4">
                    <Skeleton className="h-4 w-32" />
                    <Skeleton className="h-4 w-32" />
                  </div>
                </div>
              ))}
            </div>
          ) : error ? (
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertTitle>Error</AlertTitle>
              <AlertDescription>
                Failed to load templates. Please try again later.
              </AlertDescription>
            </Alert>
          ) : (
            <TemplateList templates={templates} />
          )}
        </CardContent>
      </Card>
    </div>
  )
}
