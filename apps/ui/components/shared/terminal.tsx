"use client"

import { useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Copy, Maximize2, Check } from "lucide-react"
import dynamic from "next/dynamic"

const XTermComponent = dynamic(() => import("./xterm-component"), { ssr: false })

interface TerminalProps {
  vmId: string
  credentials?: {
    username: string
    password: string
  }
}

export function Terminal({ vmId, credentials }: TerminalProps) {
  const [copiedField, setCopiedField] = useState<string | null>(null)

  const copyToClipboard = (text: string, field: string) => {
    navigator.clipboard.writeText(text)
    setCopiedField(field)
    setTimeout(() => setCopiedField(null), 2000)
  }

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <div>
          <CardTitle>Terminal</CardTitle>
          {credentials && (
            <div className="mt-2 flex items-center gap-4 text-sm flex-wrap">
              <div className="flex items-center gap-2">
                <span className="text-muted-foreground">Username:</span>
                <code className="bg-muted px-2 py-1 rounded">{credentials.username}</code>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6"
                  onClick={() => copyToClipboard(credentials.username, "username")}
                >
                  {copiedField === "username" ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
                </Button>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-muted-foreground">Password:</span>
                <code className="bg-muted px-2 py-1 rounded">{credentials.password}</code>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6"
                  onClick={() => copyToClipboard(credentials.password, "password")}
                >
                  {copiedField === "password" ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
                </Button>
              </div>
            </div>
          )}
        </div>
        <Button variant="ghost" size="icon">
          <Maximize2 className="h-4 w-4" />
        </Button>
      </CardHeader>
      <CardContent>
        <XTermComponent vmId={vmId} />
      </CardContent>
    </Card>
  )
}
