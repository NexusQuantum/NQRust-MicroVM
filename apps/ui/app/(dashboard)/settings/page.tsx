import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Key, User, Info, Settings2 } from "lucide-react"

export default function SettingsPage() {
  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-slate-50 to-slate-100/50 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">Settings</h1>
            <p className="mt-2 text-muted-foreground">Manage your platform configuration and preferences</p>
          </div>
          <div className="hidden lg:block">
            <div className="flex h-48 w-48 items-center justify-center rounded-2xl bg-gradient-to-br from-slate-100 to-slate-200 shadow-lg">
              <Settings2 className="h-24 w-24 text-slate-400" />
            </div>
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-slate-400/30 to-slate-600/30 blur-3xl" />
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <div className="rounded-lg bg-orange-500/10 p-2">
              <Key className="h-5 w-5 text-orange-600" />
            </div>
            <div>
              <CardTitle>API Configuration</CardTitle>
              <CardDescription>Connection details for the backend API</CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>API Endpoint</Label>
            <Input value="http://localhost:8080" readOnly className="bg-muted" />
          </div>
          <div className="space-y-2">
            <Label>WebSocket URL</Label>
            <Input value="ws://localhost:8080" readOnly className="bg-muted" />
          </div>
          <div className="space-y-2">
            <Label>Authentication Token</Label>
            <div className="flex gap-2">
              <Input value="sk-***************************" readOnly className="bg-muted" />
              <Button variant="outline">Generate</Button>
              <Button variant="outline">Revoke</Button>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <div className="rounded-lg bg-blue-500/10 p-2">
              <User className="h-5 w-5 text-blue-600" />
            </div>
            <div>
              <CardTitle>User Preferences</CardTitle>
              <CardDescription>Customize your experience</CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="theme">Theme</Label>
            <Select defaultValue="light">
              <SelectTrigger id="theme">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="light">Light</SelectItem>
                <SelectItem value="dark">Dark</SelectItem>
                <SelectItem value="auto">Auto</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-2">
            <Label htmlFor="timezone">Timezone</Label>
            <Select defaultValue="utc">
              <SelectTrigger id="timezone">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="utc">UTC</SelectItem>
                <SelectItem value="est">EST</SelectItem>
                <SelectItem value="pst">PST</SelectItem>
                <SelectItem value="cet">CET</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-2">
            <Label htmlFor="date-format">Date Format</Label>
            <Select defaultValue="iso">
              <SelectTrigger id="date-format">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="iso">ISO (2024-01-15)</SelectItem>
                <SelectItem value="us">US (01/15/2024)</SelectItem>
                <SelectItem value="eu">EU (15/01/2024)</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <div className="rounded-lg bg-green-500/10 p-2">
              <Info className="h-5 w-5 text-green-600" />
            </div>
            <div>
              <CardTitle>System Information</CardTitle>
              <CardDescription>Platform status and details</CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <dl className="grid grid-cols-2 gap-4">
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Manager Version</dt>
              <dd className="mt-1 text-sm font-semibold">v1.2.3</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Database Status</dt>
              <dd className="mt-1">
                <Badge variant="outline" className="bg-emerald-100 text-emerald-700 border-emerald-200">
                  Connected
                </Badge>
              </dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Total Hosts</dt>
              <dd className="mt-1 text-sm font-semibold">4</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Storage Usage</dt>
              <dd className="mt-1 text-sm font-semibold">1.2 TB / 5 TB</dd>
            </div>
          </dl>
        </CardContent>
      </Card>

      <div className="flex justify-end">
        <Button>Save Changes</Button>
      </div>
    </div>
  )
}
