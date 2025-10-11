"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { ExternalLink, BookOpen, MessageCircle, Bug, FileText } from "lucide-react"

export default function HelpPage() {
  const helpTopics = [
    {
      title: "Getting Started",
      description: "Learn the basics of creating and managing Firecracker microVMs",
      icon: BookOpen,
      topics: ["Creating your first VM", "VM configurations", "Network setup", "Storage management"]
    },
    {
      title: "API Documentation", 
      description: "Complete reference for the Firecracker REST API",
      icon: FileText,
      topics: ["VM lifecycle", "Device management", "Snapshots", "Metrics"]
    },
    {
      title: "Troubleshooting",
      description: "Common issues and their solutions",
      icon: Bug,
      topics: ["Boot failures", "Network connectivity", "Performance issues", "Error codes"]
    },
    {
      title: "Community Support",
      description: "Get help from the community and developers",
      icon: MessageCircle,
      topics: ["Discord community", "GitHub discussions", "Stack Overflow", "Documentation"]
    }
  ]

  const quickActions = [
    { label: "View API Docs", href: "https://github.com/firecracker-microvm/firecracker/blob/main/docs/api_requests/README.md", icon: ExternalLink },
    { label: "GitHub Repository", href: "https://github.com/firecracker-microvm/firecracker", icon: ExternalLink },
    { label: "Report Bug", href: "https://github.com/firecracker-microvm/firecracker/issues", icon: Bug },
    { label: "Join Discord", href: "#", icon: MessageCircle },
  ]

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Help & Support</h1>
        <p className="text-muted-foreground">Documentation, guides, and community resources</p>
      </div>

      {/* Quick Actions */}
      <div className="grid gap-3 md:grid-cols-2 lg:grid-cols-4">
        {quickActions.map((action) => (
          <Button key={action.label} className="h-auto p-4 bg-primary text-primary-foreground hover:outline hover:outline-2 hover:outline-offset-2 hover:[outline-color:hsl(var(--success))]" asChild>
            <a href={action.href} target="_blank" rel="noopener noreferrer">
              <div className="flex items-center gap-2">
                <action.icon className="h-4 w-4" />
                {action.label}
              </div>
            </a>
          </Button>
        ))}
      </div>

      {/* Help Topics */}
      <div className="grid gap-6 md:grid-cols-2">
        {helpTopics.map((topic) => (
          <Card key={topic.title}>
            <CardHeader>
              <div className="flex items-start gap-3">
                <div className="p-2 bg-primary/10 rounded-lg">
                  <topic.icon className="h-5 w-5 text-primary" />
                </div>
                <div className="flex-1">
                  <CardTitle className="text-lg">{topic.title}</CardTitle>
                  <p className="text-sm text-muted-foreground mt-1">{topic.description}</p>
                </div>
              </div>
            </CardHeader>
            <CardContent className="space-y-2">
              {topic.topics.map((subtopic) => (
                <div key={subtopic} className="flex items-center gap-2">
                  <Badge variant="outline" className="text-xs">
                    {subtopic}
                  </Badge>
                </div>
              ))}
            </CardContent>
          </Card>
        ))}
      </div>

      {/* System Information */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <FileText className="h-5 w-5" />
            System Information
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="grid gap-3 md:grid-cols-2">
            <div>
              <h4 className="text-sm font-medium text-muted-foreground">Version</h4>
              <p className="text-sm">NexusRust v1.0.0</p>
            </div>
            <div>
              <h4 className="text-sm font-medium text-muted-foreground">Firecracker Version</h4>
              <p className="text-sm">v1.14.0</p>
            </div>
            <div>
              <h4 className="text-sm font-medium text-muted-foreground">Platform</h4>
              <p className="text-sm">Linux x86_64</p>
            </div>
            <div>
              <h4 className="text-sm font-medium text-muted-foreground">Runtime</h4>
              <p className="text-sm">Deno 2.4.5 + Next.js 15</p>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}