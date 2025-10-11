import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Globe, Palette } from "lucide-react"

export default function SettingsPage() {
  return (
    <div className="container mx-auto py-6">
      <div className="space-y-6">
        <div>
          <h1 className="text-3xl font-bold">Settings</h1>
          <p className="text-muted-foreground">
            Configure your NexusRust MicroVM preferences
          </p>
        </div>

        <div className="grid gap-6 md:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <span className="p-2 rounded-lg bg-primary/10"><Globe className="h-4 w-4 text-primary" /></span>
                API Configuration
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                <div>
                  <label className="text-sm font-medium">Façade Base URL</label>
                  <div className="text-sm text-muted-foreground">
                    {process.env.NEXT_PUBLIC_API_BASE_URL || '/api'}
                  </div>
                </div>
                <div>
                  <label className="text-sm font-medium">WebSocket Base URL</label>
                  <div className="text-sm text-muted-foreground">
                    {process.env.NEXT_PUBLIC_WS_BASE_URL || 'ws://localhost:8000'}
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <span className="p-2 rounded-lg bg-primary/10"><Palette className="h-4 w-4 text-primary" /></span>
                Theme & Appearance
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                <div>
                  <label className="text-sm font-medium">Brand Preset</label>
                  <div className="text-sm text-muted-foreground">
                    {process.env.NEXT_PUBLIC_BRAND_PRESET || 'dark'}
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  )
}