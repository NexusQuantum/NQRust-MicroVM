"use client"

import { useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Switch } from "@/components/ui/switch"
import { Label } from "@/components/ui/label"
import { Download } from "lucide-react"

interface ContainerLogsProps {
  containerId: string
}

// Mock logs
const mockLogs = [
  "[2024-01-15 10:23:45] PostgreSQL Database directory appears to contain a database; Skipping initialization",
  "[2024-01-15 10:23:45] LOG:  starting PostgreSQL 15.3 on x86_64-pc-linux-gnu",
  "[2024-01-15 10:23:45] LOG:  listening on IPv4 address '0.0.0.0', port 5432",
  "[2024-01-15 10:23:45] LOG:  listening on IPv6 address '::', port 5432",
  "[2024-01-15 10:23:45] LOG:  database system was shut down at 2024-01-15 10:20:12 UTC",
  "[2024-01-15 10:23:45] LOG:  database system is ready to accept connections",
  "[2024-01-15 10:24:12] LOG:  connection received: host=172.17.0.1 port=54321",
  "[2024-01-15 10:24:12] LOG:  connection authorized: user=admin database=myapp",
  "[2024-01-15 10:25:33] LOG:  checkpoint starting: time",
  "[2024-01-15 10:25:35] LOG:  checkpoint complete: wrote 42 buffers (0.3%); 0 WAL file(s) added",
]

export function ContainerLogs({ containerId }: ContainerLogsProps) {
  const [autoScroll, setAutoScroll] = useState(true)

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>Container Logs</CardTitle>
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <Switch id="auto-scroll" checked={autoScroll} onCheckedChange={setAutoScroll} />
            <Label htmlFor="auto-scroll" className="text-sm">
              Auto-scroll
            </Label>
          </div>
          <Button variant="outline" size="sm">
            <Download className="mr-2 h-4 w-4" />
            Download
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <div className="bg-muted rounded-lg p-4 font-mono text-xs space-y-1 max-h-[600px] overflow-auto">
          {mockLogs.map((log, i) => (
            <div key={i} className="text-foreground">
              {log}
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  )
}
