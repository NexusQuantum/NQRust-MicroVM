'use client'

import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Separator } from "@/components/ui/separator"
import { Input } from "@/components/ui/input"
import {
  Settings2,
  Palette,
  ScrollText,
  Server,
  Info,
  Globe,
  Clock,
  Cpu,
  HardDrive,
  Zap,
  Database,
  User,
  Lock,
  Trash2,
  Eye,
  EyeOff,
  Copy,
  Check,
  ChevronLeft,
  ChevronRight,
  RefreshCw,
  CheckCircle,
  XCircle,
  Monitor,
  Code,
  Box
} from "lucide-react"
import { useTheme } from "next-themes"
import {
  useHosts,
  useVMs,
  useContainers,
  useFunctions,
  usePreferences,
  useUpdatePreferences,
  useProfile,
  useUpdateProfile,
  useChangePassword,
  useUploadAvatar,
  useDeleteAvatar,
  useAuditLogs,
  useDbInfo,
  useSystemStats
} from "@/lib/queries"
import { useState, useEffect, useMemo, useCallback } from "react"
import { toast } from "sonner"
import { AvatarUpload } from "@/components/user"
import { useDateFormat } from "@/lib/hooks/use-date-format"
import { useAuthStore } from "@/lib/auth/store"
import type { AuditLogQueryParams } from "@/lib/types"

const ACTION_GROUPS: Record<string, string[]> = {
  "All": [],
  "Auth": ["login", "logout", "login_failed"],
  "VM": ["create_vm", "start_vm", "stop_vm", "pause_vm", "resume_vm", "delete_vm", "update_vm"],
  "Container": ["create_container", "start_container", "stop_container", "delete_container"],
  "Function": ["create_function", "invoke_function", "update_function", "delete_function"],
  "System": ["system_event"],
}

function formatAction(action: string): string {
  return action.replace(/_/g, " ").replace(/\b\w/g, c => c.toUpperCase())
}

function LoggingTabContent() {
  const dateFormat = useDateFormat()
  const [actionFilter, setActionFilter] = useState("All")
  const [page, setPage] = useState(0)
  const [copied, setCopied] = useState(false)
  const pageSize = 15

  const params: AuditLogQueryParams = useMemo(() => {
    const p: AuditLogQueryParams = { limit: pageSize, offset: page * pageSize }
    const group = ACTION_GROUPS[actionFilter]
    if (group && group.length === 1) {
      p.action = group[0]
    }
    // For multi-action groups, we filter client-side from a larger fetch
    if (group && group.length > 1) {
      p.limit = 200
      p.offset = 0
    }
    return p
  }, [actionFilter, page])

  const { data: logsData, isLoading: logsLoading, refetch: refetchLogs } = useAuditLogs(params, 15000)
  const { data: dbInfo } = useDbInfo()
  const { data: stats } = useSystemStats(15000)

  // Client-side filter for multi-action groups, then paginate
  const filteredItems = useMemo(() => {
    if (!logsData?.items) return []
    const group = ACTION_GROUPS[actionFilter]
    if (!group || group.length <= 1) return logsData.items
    return group.length > 0
      ? logsData.items.filter(item => group.includes(item.action))
      : logsData.items
  }, [logsData, actionFilter])

  const paginatedItems = useMemo(() => {
    const group = ACTION_GROUPS[actionFilter]
    if (group && group.length > 1) {
      return filteredItems.slice(page * pageSize, (page + 1) * pageSize)
    }
    return filteredItems
  }, [filteredItems, actionFilter, page])

  const totalCount = useMemo(() => {
    const group = ACTION_GROUPS[actionFilter]
    if (group && group.length > 1) return filteredItems.length
    return logsData?.total ?? 0
  }, [logsData, filteredItems, actionFilter])

  const totalPages = Math.max(1, Math.ceil(totalCount / pageSize))

  const handleCopy = useCallback((text: string) => {
    navigator.clipboard.writeText(text)
    setCopied(true)
    toast.success("Copied to clipboard")
    setTimeout(() => setCopied(false), 2000)
  }, [])

  return (
    <>
      {/* System Overview Card */}
      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <div className="rounded-lg bg-blue-500/10 p-2">
              <Server className="h-5 w-5 text-blue-600 dark:text-blue-400" />
            </div>
            <div>
              <CardTitle>System Overview</CardTitle>
              <CardDescription>Current resource counts across the platform</CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div className="flex items-center gap-3 rounded-lg border p-3">
              <Server className="h-5 w-5 text-muted-foreground" />
              <div>
                <p className="text-2xl font-bold">{stats?.total_hosts ?? "—"}</p>
                <p className="text-xs text-muted-foreground">Hosts</p>
              </div>
            </div>
            <div className="flex items-center gap-3 rounded-lg border p-3">
              <Monitor className="h-5 w-5 text-muted-foreground" />
              <div>
                <p className="text-2xl font-bold">
                  {stats ? `${stats.running_vms}/${stats.total_vms}` : "—"}
                </p>
                <p className="text-xs text-muted-foreground">VMs (running/total)</p>
              </div>
            </div>
            <div className="flex items-center gap-3 rounded-lg border p-3">
              <Code className="h-5 w-5 text-muted-foreground" />
              <div>
                <p className="text-2xl font-bold">{stats?.total_functions ?? "—"}</p>
                <p className="text-xs text-muted-foreground">Functions</p>
              </div>
            </div>
            <div className="flex items-center gap-3 rounded-lg border p-3">
              <Box className="h-5 w-5 text-muted-foreground" />
              <div>
                <p className="text-2xl font-bold">
                  {stats ? `${stats.running_containers}/${stats.total_containers}` : "—"}
                </p>
                <p className="text-xs text-muted-foreground">Containers (running/total)</p>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Activity Logs Card */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <div className="rounded-lg bg-purple-500/10 p-2">
                <ScrollText className="h-5 w-5 text-purple-600 dark:text-purple-400" />
              </div>
              <div>
                <CardTitle>Activity Logs</CardTitle>
                <CardDescription>Audit trail of all system operations</CardDescription>
              </div>
            </div>
            <Button variant="outline" size="sm" onClick={() => refetchLogs()}>
              <RefreshCw className="mr-2 h-3 w-3" />
              Refresh
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Filters */}
          <div className="flex items-center gap-3">
            <Select value={actionFilter} onValueChange={(val) => { setActionFilter(val); setPage(0) }}>
              <SelectTrigger className="w-[160px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {Object.keys(ACTION_GROUPS).map(group => (
                  <SelectItem key={group} value={group}>{group}</SelectItem>
                ))}
              </SelectContent>
            </Select>
            <span className="text-sm text-muted-foreground">
              {totalCount} {totalCount === 1 ? "entry" : "entries"}
            </span>
          </div>

          {/* Log Table */}
          <div className="rounded-md border">
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b bg-muted/50">
                    <th className="px-4 py-2 text-left font-medium">Time</th>
                    <th className="px-4 py-2 text-left font-medium">User</th>
                    <th className="px-4 py-2 text-left font-medium">Action</th>
                    <th className="px-4 py-2 text-left font-medium">Resource</th>
                    <th className="px-4 py-2 text-left font-medium">Status</th>
                  </tr>
                </thead>
                <tbody>
                  {logsLoading ? (
                    <tr>
                      <td colSpan={5} className="px-4 py-8 text-center text-muted-foreground">
                        Loading...
                      </td>
                    </tr>
                  ) : paginatedItems.length === 0 ? (
                    <tr>
                      <td colSpan={5} className="px-4 py-8 text-center text-muted-foreground">
                        No audit logs found. Operations like creating VMs, containers, and functions will appear here.
                      </td>
                    </tr>
                  ) : (
                    paginatedItems.map((log) => (
                      <tr key={log.id} className="border-b last:border-0 hover:bg-muted/30">
                        <td className="px-4 py-2 text-xs text-muted-foreground whitespace-nowrap">
                          {dateFormat.formatRelative(log.created_at)}
                        </td>
                        <td className="px-4 py-2 font-medium">{log.username}</td>
                        <td className="px-4 py-2">
                          <Badge variant="outline" className="text-xs font-mono">
                            {formatAction(log.action)}
                          </Badge>
                        </td>
                        <td className="px-4 py-2 text-xs text-muted-foreground">
                          {log.resource_type && (
                            <span className="capitalize">{log.resource_type}</span>
                          )}
                          {log.resource_id && (
                            <span className="ml-1 font-mono text-[10px]">
                              {log.resource_id.slice(0, 8)}
                            </span>
                          )}
                        </td>
                        <td className="px-4 py-2">
                          {log.success ? (
                            <CheckCircle className="h-4 w-4 text-green-500" />
                          ) : (
                            <span className="flex items-center gap-1">
                              <XCircle className="h-4 w-4 text-red-500" />
                              {log.error_message && (
                                <span className="text-xs text-red-500 truncate max-w-[120px]" title={log.error_message}>
                                  {log.error_message}
                                </span>
                              )}
                            </span>
                          )}
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          </div>

          {/* Pagination */}
          {totalPages > 1 && (
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">
                Page {page + 1} of {totalPages}
              </span>
              <div className="flex gap-2">
                <Button variant="outline" size="sm" disabled={page === 0} onClick={() => setPage(p => p - 1)}>
                  <ChevronLeft className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" disabled={page >= totalPages - 1} onClick={() => setPage(p => p + 1)}>
                  <ChevronRight className="h-4 w-4" />
                </Button>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Database Connection Card */}
      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <div className="rounded-lg bg-blue-500/10 p-2">
              <Database className="h-5 w-5 text-blue-600 dark:text-blue-400" />
            </div>
            <div>
              <CardTitle>Database Connection</CardTitle>
              <CardDescription>Connect with external tools (psql, pgAdmin, DBeaver)</CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          {dbInfo ? (
            <>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1">
                  <Label className="text-xs text-muted-foreground">Host</Label>
                  <p className="text-sm font-mono">{dbInfo.host}</p>
                </div>
                <div className="space-y-1">
                  <Label className="text-xs text-muted-foreground">Port</Label>
                  <p className="text-sm font-mono">{dbInfo.port}</p>
                </div>
                <div className="space-y-1">
                  <Label className="text-xs text-muted-foreground">Database</Label>
                  <p className="text-sm font-mono">{dbInfo.database}</p>
                </div>
                <div className="space-y-1">
                  <Label className="text-xs text-muted-foreground">Username</Label>
                  <p className="text-sm font-mono">{dbInfo.username}</p>
                </div>
              </div>

              <Separator />

              <div className="space-y-2">
                <Label className="text-xs text-muted-foreground">Connection String (password masked)</Label>
                <div className="flex items-center gap-2">
                  <code className="flex-1 rounded bg-muted px-3 py-2 text-xs font-mono break-all">
                    {dbInfo.connection_string_masked}
                  </code>
                  <Button
                    variant="outline"
                    size="icon"
                    className="shrink-0"
                    onClick={() => handleCopy(dbInfo.connection_string_masked)}
                  >
                    {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
                  </Button>
                </div>
              </div>

              <div className="rounded-md bg-muted/50 border p-3">
                <p className="text-xs text-muted-foreground">
                  Query the <code className="font-mono bg-muted px-1 rounded">audit.audit_logs</code> table to view all activity.
                  Example: <code className="font-mono bg-muted px-1 rounded">SELECT * FROM audit.audit_logs ORDER BY created_at DESC LIMIT 50;</code>
                </p>
              </div>
            </>
          ) : (
            <p className="text-sm text-muted-foreground">Loading connection info...</p>
          )}
        </CardContent>
      </Card>
    </>
  )
}

export default function SettingsPage() {
  const { theme, setTheme } = useTheme()
  const { user: authUser, avatarRefreshKey, refreshAvatar } = useAuthStore() // Get current user from auth store
  const [mounted, setMounted] = useState(false)
  const dateFormat = useDateFormat()

  // Fetch real data
  const { data: hosts } = useHosts()
  const { data: vms } = useVMs(false) // Only user-facing VMs (exclude internal VMs for functions/containers)
  const { data: containers } = useContainers()
  const { data: functions } = useFunctions()

  // User preferences and profile
  const { data: preferences, isLoading: prefsLoading } = usePreferences()
  const { data: profile, isLoading: profileLoading } = useProfile()
  const updatePreferencesMutation = useUpdatePreferences()
  const updateProfileMutation = useUpdateProfile()
  const changePasswordMutation = useChangePassword()
  const uploadAvatarMutation = useUploadAvatar()
  const deleteAvatarMutation = useDeleteAvatar()

  // Local state for form inputs (synced with backend)
  const [localTimezone, setLocalTimezone] = useState("UTC")
  const [localDateFormat, setLocalDateFormat] = useState("iso")
  const [localVmDefaults, setLocalVmDefaults] = useState({
    vcpu: 2,
    mem_mib: 2048,
    disk_gb: 10
  })
  const [localAutoRefresh, setLocalAutoRefresh] = useState<number | undefined>(30)
  const [localMetricsRetention, setLocalMetricsRetention] = useState<number | undefined>(7)

  // Profile form state
  const [newUsername, setNewUsername] = useState("")
  const [currentPassword, setCurrentPassword] = useState("")
  const [newPassword, setNewPassword] = useState("")
  const [confirmPassword, setConfirmPassword] = useState("")

  // Password visibility state
  const [showCurrentPassword, setShowCurrentPassword] = useState(false)
  const [showNewPassword, setShowNewPassword] = useState(false)
  const [showConfirmPassword, setShowConfirmPassword] = useState(false)

  // Local date formatter based on local state (for preview)
  const formatDateWithLocalFormat = useMemo(() => {
    return (date: Date | string) => {
      const d = typeof date === 'string' ? new Date(date) : date
      if (isNaN(d.getTime())) return 'Invalid Date'

      const format = localDateFormat || "iso"

      try {
        switch (format) {
          case 'us':
            // US format: MM/DD/YYYY
            return d.toLocaleDateString('en-US', {
              month: '2-digit',
              day: '2-digit',
              year: 'numeric'
            })
          case 'eu':
            // EU format: DD/MM/YYYY
            return d.toLocaleDateString('en-GB', {
              day: '2-digit',
              month: '2-digit',
              year: 'numeric'
            })
          case 'iso':
          default:
            // ISO format: YYYY-MM-DD
            return d.toISOString().split('T')[0]
        }
      } catch {
        return d.toISOString().split('T')[0]
      }
    }
  }, [localDateFormat])

  const formatDateTimeWithLocalFormat = useMemo(() => {
    return (date: Date | string) => {
      const d = typeof date === 'string' ? new Date(date) : date
      if (isNaN(d.getTime())) return 'Invalid Date'

      const dateStr = formatDateWithLocalFormat(d)
      const timeStr = d.toLocaleTimeString('en-US', {
        hour12: false,
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit'
      })

      return `${dateStr} ${timeStr}`
    }
  }, [formatDateWithLocalFormat])

  // Sync preferences from backend to local state
  useEffect(() => {
    if (preferences) {
      setLocalTimezone(preferences.timezone || "UTC")
      setLocalDateFormat(preferences.date_format || "iso")
      setLocalVmDefaults(preferences.vm_defaults)
      setLocalAutoRefresh(preferences.auto_refresh)
      setLocalMetricsRetention(preferences.metrics_retention)
    }
  }, [preferences])

  // Sync username from auth store (always up-to-date)
  useEffect(() => {
    if (authUser) {
      setNewUsername(authUser.username)
    }
  }, [authUser])

  useEffect(() => {
    setMounted(true)
  }, [])

  const handleSavePreferences = () => {
    updatePreferencesMutation.mutate({
      timezone: localTimezone,
      date_format: localDateFormat,
      // notifications: localNotifications, // TODO: Uncomment when notification system is implemented
      vm_defaults: localVmDefaults,
      auto_refresh: localAutoRefresh,
      // metrics_retention: localMetricsRetention, // TODO: Uncomment when metrics storage is implemented
    })
  }

  const handleUpdateProfile = () => {
    if (newUsername !== authUser?.username) {
      updateProfileMutation.mutate({ username: newUsername }, {
        onSuccess: () => {
          toast.success("Profile Updated", {
            description: "Your profile has been updated successfully",
          })
        },
        onError: (error: Error) => {
          toast.error("Update Failed", {
            description: error.message || "Failed to update profile",
          })
        }
      })
    }
  }

  const handleChangePassword = () => {
    if (!currentPassword || !newPassword || !confirmPassword) {
      toast.error("Validation Error", {
        description: "Please fill in all password fields",
      })
      return
    }
    if (newPassword !== confirmPassword) {
      toast.error("Validation Error", {
        description: "New passwords do not match",
      })
      return
    }
    if (newPassword.length < 8) {
      toast.error("Validation Error", {
        description: "Password must be at least 8 characters",
      })
      return
    }

    changePasswordMutation.mutate({
      current_password: currentPassword,
      new_password: newPassword,
    }, {
      onSuccess: () => {
        setCurrentPassword("")
        setNewPassword("")
        setConfirmPassword("")
        toast.success("Password Changed", {
          description: "Your password has been changed successfully",
        })
      },
      onError: (error: Error) => {
        toast.error("Change Password Failed", {
          description: "Failed to change password. Please check your current password.",
        })
      }
    })
  }

  const handleAvatarUpload = (file: File) => {
    uploadAvatarMutation.mutate(file, {
      onSuccess: () => {
        refreshAvatar() // Force avatar refresh globally
        toast.success("Avatar Uploaded", {
          description: "Your avatar has been uploaded successfully",
        })
      },
      onError: (error: Error) => {
        toast.error("Upload Failed", {
          description: error.message || "Failed to upload avatar",
        })
      }
    })
  }

  const handleDeleteAvatar = () => {
    if (confirm("Are you sure you want to delete your avatar?")) {
      deleteAvatarMutation.mutate(undefined, {
        onSuccess: () => {
          refreshAvatar() // Force avatar refresh globally
          toast.success("Avatar Deleted", {
            description: "Your avatar has been deleted successfully",
          })
        },
        onError: (error: Error) => {
          toast.error("Delete Failed", {
            description: error.message || "Failed to delete avatar",
          })
        }
      })
    }
  }

  const handleResetPreferences = () => {
    if (confirm("Are you sure you want to reset all preferences to defaults?")) {
      // Reset to default values
      updatePreferencesMutation.mutate({
        timezone: "UTC",
        date_format: "iso",
        notifications: { email: true, browser: true, desktop: false },
        vm_defaults: { vcpu: 2, mem_mib: 2048, disk_gb: 10 },
        auto_refresh: 30,
        metrics_retention: 7,
      })
    }
  }

  if (!mounted) {
    return null // Avoid hydration mismatch
  }

  // Calculate storage usage (rough estimate)
  const totalVMs = vms?.length || 0
  const totalContainers = containers?.length || 0
  const totalFunctions = functions?.length || 0
  const estimatedStorage = (totalVMs * 2 + totalContainers * 1 + totalFunctions * 0.5).toFixed(1)

  const isLoading = prefsLoading || profileLoading

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-slate-50 to-slate-100/50 dark:from-slate-900 dark:to-slate-800/50 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">Settings</h1>
            <p className="mt-2 text-muted-foreground">Manage your platform configuration and preferences</p>
          </div>
          <div className="hidden lg:block">
            <div className="flex h-48 w-48 items-center justify-center rounded-2xl bg-gradient-to-br from-slate-100 to-slate-200 dark:from-slate-800 dark:to-slate-700 shadow-lg">
              <Settings2 className="h-24 w-24 text-slate-400 dark:text-slate-500" />
            </div>
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-slate-400/30 to-slate-600/30 blur-3xl" />
      </div>

      <Tabs defaultValue="account" className="space-y-6">
        <TabsList className="grid w-full grid-cols-5">
          <TabsTrigger value="account">
            <User className="mr-2 h-4 w-4" />
            Account
          </TabsTrigger>
          <TabsTrigger value="appearance">
            <Palette className="mr-2 h-4 w-4" />
            Appearance
          </TabsTrigger>
          <TabsTrigger value="logging">
            <ScrollText className="mr-2 h-4 w-4" />
            Logging
          </TabsTrigger>
          <TabsTrigger value="defaults">
            <Server className="mr-2 h-4 w-4" />
            Defaults
          </TabsTrigger>
          <TabsTrigger value="system">
            <Info className="mr-2 h-4 w-4" />
            System
          </TabsTrigger>
        </TabsList>

        {/* Account Tab */}
        <TabsContent value="account" className="space-y-6">
          <Card>
            <CardHeader>
              <div className="flex items-center gap-2">
                <div className="rounded-lg bg-indigo-500/10 p-2">
                  <User className="h-5 w-5 text-indigo-600 dark:text-indigo-400" />
                </div>
                <div>
                  <CardTitle>Profile Information</CardTitle>
                  <CardDescription>Update your profile and avatar</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="flex flex-col items-center gap-6 sm:flex-row sm:items-start">
                <AvatarUpload
                  onUpload={handleAvatarUpload}
                  currentAvatarPath={profile?.avatar_path}
                  username={authUser?.username}
                  isUploading={uploadAvatarMutation.isPending}
                  refreshKey={avatarRefreshKey}
                />

                <div className="flex-1 space-y-4 w-full">
                  <div className="space-y-2">
                    <Label htmlFor="username">Username</Label>
                    <Input
                      id="username"
                      value={newUsername}
                      onChange={(e) => setNewUsername(e.target.value)}
                      placeholder="Enter username"
                      disabled
                    />
                  </div>

                  <div className="space-y-2">
                    <Label>Role</Label>
                    <div className="flex items-center gap-2">
                      <Badge variant="outline" className="capitalize">
                        {authUser?.role || "user"}
                      </Badge>
                    </div>
                  </div>

                  <div className="space-y-2">
                    <Label>Account Created</Label>
                    <p className="text-sm text-muted-foreground">
                      {profile?.created_at ? formatDateWithLocalFormat(profile.created_at) : "N/A"}
                    </p>
                  </div>

                  <div className="space-y-2">
                    <Label>Last Login</Label>
                    <p className="text-sm text-muted-foreground">
                      {profile?.last_login_at ? formatDateTimeWithLocalFormat(profile.last_login_at) : "Never"}
                    </p>
                  </div>

                  <div className="flex gap-2">
                    <Button
                      onClick={handleUpdateProfile}
                      disabled={isLoading || newUsername === authUser?.username || updateProfileMutation.isPending}
                    >
                      Update Profile
                    </Button>
                    {profile?.avatar_path && (
                      <Button
                        variant="outline"
                        onClick={handleDeleteAvatar}
                        disabled={deleteAvatarMutation.isPending}
                      >
                        <Trash2 className="mr-2 h-4 w-4" />
                        Remove Avatar
                      </Button>
                    )}
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <div className="flex items-center gap-2">
                <div className="rounded-lg bg-red-500/10 p-2">
                  <Lock className="h-5 w-5 text-red-600 dark:text-red-400" />
                </div>
                <div>
                  <CardTitle>Change Password</CardTitle>
                  <CardDescription>Update your account password</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="current-password">Current Password</Label>
                <div className="relative">
                  <Input
                    id="current-password"
                    type={showCurrentPassword ? "text" : "password"}
                    value={currentPassword}
                    onChange={(e) => setCurrentPassword(e.target.value)}
                    placeholder="Enter current password"
                    className="pr-10"
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="absolute right-0 top-0 h-full px-3 py-2 hover:bg-transparent"
                    onClick={() => setShowCurrentPassword(!showCurrentPassword)}
                  >
                    {showCurrentPassword ? (
                      <Eye className="h-4 w-4 text-muted-foreground" />
                    ) : (
                      <EyeOff className="h-4 w-4 text-muted-foreground" />
                    )}
                  </Button>
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="new-password">New Password</Label>
                <div className="relative">
                  <Input
                    id="new-password"
                    type={showNewPassword ? "text" : "password"}
                    value={newPassword}
                    onChange={(e) => setNewPassword(e.target.value)}
                    placeholder="Enter new password (min 8 characters)"
                    className="pr-10"
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="absolute right-0 top-0 h-full px-3 py-2 hover:bg-transparent"
                    onClick={() => setShowNewPassword(!showNewPassword)}
                  >
                    {showNewPassword ? (
                      <Eye className="h-4 w-4 text-muted-foreground" />
                    ) : (
                      <EyeOff className="h-4 w-4 text-muted-foreground" />
                    )}
                  </Button>
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="confirm-password">Confirm New Password</Label>
                <div className="relative">
                  <Input
                    id="confirm-password"
                    type={showConfirmPassword ? "text" : "password"}
                    value={confirmPassword}
                    onChange={(e) => setConfirmPassword(e.target.value)}
                    placeholder="Confirm new password"
                    className="pr-10"
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="absolute right-0 top-0 h-full px-3 py-2 hover:bg-transparent"
                    onClick={() => setShowConfirmPassword(!showConfirmPassword)}
                  >
                    {showConfirmPassword ? (
                      <Eye className="h-4 w-4 text-muted-foreground" />
                    ) : (
                      <EyeOff className="h-4 w-4 text-muted-foreground" />
                    )}
                  </Button>
                </div>
              </div>

              <Button
                onClick={handleChangePassword}
                disabled={changePasswordMutation.isPending || !currentPassword || !newPassword || !confirmPassword}
              >
                {changePasswordMutation.isPending ? "Changing..." : "Change Password"}
              </Button>
            </CardContent>
          </Card>
        </TabsContent>

        {/* Appearance Tab */}
        <TabsContent value="appearance" className="space-y-6">
          <Card>
            <CardHeader>
              <div className="flex items-center gap-2">
                <div className="rounded-lg bg-purple-500/10 p-2">
                  <Palette className="h-5 w-5 text-purple-600 dark:text-purple-400" />
                </div>
                <div>
                  <CardTitle>Theme Preferences</CardTitle>
                  <CardDescription>Customize the visual appearance of the application</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="space-y-2">
                <Label htmlFor="theme">Color Theme</Label>
                <Select value={theme || 'system'} onValueChange={setTheme}>
                  <SelectTrigger id="theme">
                    <SelectValue placeholder="Select theme" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="light">Light</SelectItem>
                    <SelectItem value="dark">Dark</SelectItem>
                    <SelectItem value="system">System</SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-sm text-muted-foreground">
                  Choose your preferred color theme. System will match your OS settings. <strong>Current: {theme || 'system'}</strong>
                </p>
                <div className="rounded-md bg-blue-50 dark:bg-blue-950 p-3 text-sm text-blue-800 dark:text-blue-200">
                  Theme changes are automatically saved and synced with your profile.
                </div>
              </div>

              <Separator />

              <div className="space-y-2">
                <Label htmlFor="timezone">Timezone</Label>
                <Select
                  value={localTimezone}
                  onValueChange={setLocalTimezone}
                  disabled={isLoading}
                >
                  <SelectTrigger id="timezone">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="UTC">UTC (Coordinated Universal Time)</SelectItem>
                    <SelectItem value="America/New_York">EST (Eastern Standard Time)</SelectItem>
                    <SelectItem value="America/Los_Angeles">PST (Pacific Standard Time)</SelectItem>
                    <SelectItem value="Europe/Paris">CET (Central European Time)</SelectItem>
                    <SelectItem value="Asia/Tokyo">JST (Japan Standard Time)</SelectItem>
                    <SelectItem value="Asia/Kolkata">IST (India Standard Time)</SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-sm text-muted-foreground">
                  Your timezone for displaying dates and times
                </p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="date-format">Date Format</Label>
                <Select
                  value={localDateFormat}
                  onValueChange={setLocalDateFormat}
                  disabled={isLoading}
                >
                  <SelectTrigger id="date-format">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="iso">ISO (2024-01-15)</SelectItem>
                    <SelectItem value="us">US (01/15/2024)</SelectItem>
                    <SelectItem value="eu">EU (15/01/2024)</SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-sm text-muted-foreground">
                  How dates should be formatted throughout the application
                </p>
              </div>

              <div className="flex justify-end gap-2 pt-4">
                <Button
                  variant="outline"
                  onClick={() => {
                    setLocalTimezone(preferences?.timezone || "UTC")
                    setLocalDateFormat(preferences?.date_format || "iso")
                  }}
                  disabled={isLoading}
                >
                  Reset
                </Button>
                <Button
                  onClick={handleSavePreferences}
                  disabled={isLoading || updatePreferencesMutation.isPending}
                >
                  {updatePreferencesMutation.isPending ? "Saving..." : "Save Appearance Settings"}
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        {/* Logging Tab */}
        <TabsContent value="logging" className="space-y-6">
          <LoggingTabContent />
        </TabsContent>

        {/* Defaults Tab */}
        <TabsContent value="defaults" className="space-y-6">
          <Card>
            <CardHeader>
              <div className="flex items-center gap-2">
                <div className="rounded-lg bg-green-500/10 p-2">
                  <Server className="h-5 w-5 text-green-600 dark:text-green-400" />
                </div>
                <div>
                  <CardTitle>Default VM Configuration</CardTitle>
                  <CardDescription>Set default values for new virtual machines</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="default-vcpu">
                    <div className="flex items-center gap-2">
                      <Cpu className="h-4 w-4" />
                      Default vCPU Count
                    </div>
                  </Label>
                  <Select
                    value={localVmDefaults.vcpu.toString()}
                    onValueChange={(val) => setLocalVmDefaults({ ...localVmDefaults, vcpu: parseInt(val) })}
                    disabled={isLoading}
                  >
                    <SelectTrigger id="default-vcpu">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="1">1 vCPU</SelectItem>
                      <SelectItem value="2">2 vCPUs</SelectItem>
                      <SelectItem value="4">4 vCPUs</SelectItem>
                      <SelectItem value="8">8 vCPUs</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="default-memory">
                    <div className="flex items-center gap-2">
                      <Zap className="h-4 w-4" />
                      Default Memory (MB)
                    </div>
                  </Label>
                  <Select
                    value={localVmDefaults.mem_mib.toString()}
                    onValueChange={(val) => setLocalVmDefaults({ ...localVmDefaults, mem_mib: parseInt(val) })}
                    disabled={isLoading}
                  >
                    <SelectTrigger id="default-memory">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="512">512 MB</SelectItem>
                      <SelectItem value="1024">1 GB</SelectItem>
                      <SelectItem value="2048">2 GB</SelectItem>
                      <SelectItem value="4096">4 GB</SelectItem>
                      <SelectItem value="8192">8 GB</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="default-disk">
                  <div className="flex items-center gap-2">
                    <HardDrive className="h-4 w-4" />
                    Default Disk Size (GB)
                  </div>
                </Label>
                <Select
                  value={localVmDefaults.disk_gb.toString()}
                  onValueChange={(val) => setLocalVmDefaults({ ...localVmDefaults, disk_gb: parseInt(val) })}
                  disabled={isLoading}
                >
                  <SelectTrigger id="default-disk">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="10">10 GB</SelectItem>
                    <SelectItem value="20">20 GB</SelectItem>
                    <SelectItem value="50">50 GB</SelectItem>
                    <SelectItem value="100">100 GB</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        {/* System Tab */}
        <TabsContent value="system" className="space-y-6">
          <Card>
            <CardHeader>
              <div className="flex items-center gap-2">
                <div className="rounded-lg bg-orange-500/10 p-2">
                  <Info className="h-5 w-5 text-orange-600 dark:text-orange-600" />
                </div>
                <div>
                  <CardTitle>System Information</CardTitle>
                  <CardDescription>Platform status and configuration details</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <dl className="grid grid-cols-2 gap-6">
                <div className="space-y-1">
                  <dt className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
                    <Server className="h-4 w-4" />
                    Total Hosts
                  </dt>
                  <dd className="text-2xl font-bold">{hosts?.length || 0}</dd>
                </div>
                <div className="space-y-1">
                  <dt className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
                    <Database className="h-4 w-4" />
                    Active VMs
                  </dt>
                  <dd className="text-2xl font-bold">{totalVMs}</dd>
                </div>
                <div className="space-y-1">
                  <dt className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
                    <Globe className="h-4 w-4" />
                    Containers
                  </dt>
                  <dd className="text-2xl font-bold">{totalContainers}</dd>
                </div>
                <div className="space-y-1">
                  <dt className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
                    <Zap className="h-4 w-4" />
                    Functions
                  </dt>
                  <dd className="text-2xl font-bold">{totalFunctions}</dd>
                </div>
                <div className="space-y-1">
                  <dt className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
                    <HardDrive className="h-4 w-4" />
                    Est. Storage Used
                  </dt>
                  <dd className="text-2xl font-bold">{estimatedStorage} GB</dd>
                </div>
                <div className="space-y-1">
                  <dt className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
                    <Clock className="h-4 w-4" />
                    API Endpoint
                  </dt>
                  <dd className="mt-1">
                    <Badge variant="outline" className="bg-emerald-100 text-emerald-700 border-emerald-200 dark:bg-emerald-900/30 dark:text-emerald-400 dark:border-emerald-800">
                      Connected
                    </Badge>
                  </dd>
                </div>
              </dl>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <div className="flex items-center gap-2">
                <div className="rounded-lg bg-blue-500/10 p-2">
                  <Clock className="h-5 w-5 text-blue-600 dark:text-blue-400" />
                </div>
                <div>
                  <CardTitle>Performance Settings</CardTitle>
                  <CardDescription>Configure application behavior and data retention</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="space-y-2">
                <Label htmlFor="refresh-interval">Auto-refresh Interval (seconds)</Label>
                <Select
                  value={localAutoRefresh?.toString() || "0"}
                  onValueChange={(val) => setLocalAutoRefresh(val === "0" ? undefined : parseInt(val))}
                  disabled={isLoading}
                >
                  <SelectTrigger id="refresh-interval">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="0">Disabled</SelectItem>
                    <SelectItem value="10">10 seconds</SelectItem>
                    <SelectItem value="30">30 seconds</SelectItem>
                    <SelectItem value="60">1 minute</SelectItem>
                    <SelectItem value="300">5 minutes</SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-sm text-muted-foreground">
                  How often to refresh dashboard data automatically
                </p>
                <div className="rounded-md bg-green-50 dark:bg-green-950 p-3 text-sm text-green-800 dark:text-green-200">
                  <strong>✓ Active:</strong> Dashboard auto-refresh is now working!
                </div>
              </div>

              {/* TODO: Metrics retention not yet implemented. Uncomment when metrics storage is added. */}
              {/*
              <div className="space-y-2">
                <Label htmlFor="metrics-retention">Metrics Retention (days)</Label>
                <Select
                  value={localMetricsRetention?.toString() || "7"}
                  onValueChange={(val) => setLocalMetricsRetention(parseInt(val))}
                  disabled={isLoading}
                >
                  <SelectTrigger id="metrics-retention">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="1">1 day</SelectItem>
                    <SelectItem value="7">7 days</SelectItem>
                    <SelectItem value="30">30 days</SelectItem>
                    <SelectItem value="90">90 days</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              */}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      {/* Action Buttons */}
      <div className="flex justify-between">
        <Button variant="outline" onClick={handleResetPreferences} disabled={isLoading}>
          Reset to Defaults
        </Button>
        <Button onClick={handleSavePreferences} disabled={isLoading || updatePreferencesMutation.isPending}>
          {updatePreferencesMutation.isPending ? "Saving..." : "Save Changes"}
        </Button>
      </div>
    </div>
  )
}
