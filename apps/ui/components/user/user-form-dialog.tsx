"use client"

import { useEffect, useState } from "react"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Loader2, Eye, EyeOff, CheckCircle2, XCircle } from "lucide-react"
import type { User, CreateUserRequest, UpdateUserRequest } from "@/lib/types"

interface UserFormDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  mode: "create" | "edit"
  user?: User
  existingUsers?: User[]
  onSubmit: (data: CreateUserRequest | UpdateUserRequest) => void
  isLoading?: boolean
}

export function UserFormDialog({
  open,
  onOpenChange,
  mode,
  user,
  existingUsers = [],
  onSubmit,
  isLoading = false,
}: UserFormDialogProps) {
  const [formData, setFormData] = useState<CreateUserRequest>({
    username: "",
    email: "",
    password: "",
    role: "user",
  })

  const [errors, setErrors] = useState<Record<string, string>>({})
  const [showPassword, setShowPassword] = useState(false)
  const [usernameStatus, setUsernameStatus] = useState<"idle" | "checking" | "available" | "taken">("idle")

  useEffect(() => {
    if (mode === "edit" && user) {
      setFormData({
        username: user.username,
        email: user.email || "",
        password: "",
        role: user.role,
      })
    } else {
      setFormData({
        username: "",
        email: "",
        password: "",
        role: "user",
      })
    }
    setErrors({})
    setShowPassword(false)
    setUsernameStatus("idle")
  }, [mode, user, open])

  // Real-time username availability check with debounce
  useEffect(() => {
    const username = formData.username.trim()

    // Reset if empty
    if (!username) {
      setUsernameStatus("idle")
      return
    }

    // Skip check if editing and username unchanged
    if (mode === "edit" && user && username.toLowerCase() === user.username.toLowerCase()) {
      setUsernameStatus("idle")
      return
    }

    setUsernameStatus("checking")

    const timeoutId = setTimeout(() => {
      const isDuplicate = existingUsers.some(
        (existingUser) =>
          existingUser.username.toLowerCase() === username.toLowerCase() &&
          (mode === "create" || existingUser.id !== user?.id)
      )
      setUsernameStatus(isDuplicate ? "taken" : "available")
    }, 300)

    return () => clearTimeout(timeoutId)
  }, [formData.username, existingUsers, mode, user])

  const validateForm = () => {
    const newErrors: Record<string, string> = {}

    if (!formData.username.trim()) {
      newErrors.username = "Username is required"
    } else {
      // Check for duplicate username
      const isDuplicate = existingUsers.some(
        (existingUser) =>
          existingUser.username.toLowerCase() === formData.username.toLowerCase() &&
          (mode === "create" || existingUser.id !== user?.id)
      )

      if (isDuplicate) {
        newErrors.username = "Username already exists. Please choose a different username."
      }
    }

    if (mode === "create" && !formData.password) {
      newErrors.password = "Password is required"
    }

    if (formData.password && formData.password.length < 8) {
      newErrors.password = "Password must be at least 8 characters"
    }

    setErrors(newErrors)
    return Object.keys(newErrors).length === 0
  }

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()

    if (!validateForm()) {
      return
    }

    if (mode === "edit") {
      // For edit, only send changed fields
      const updateData: UpdateUserRequest = {
        username: formData.username !== user?.username ? formData.username : undefined,
        role: formData.role !== user?.role ? formData.role : undefined,
        password: formData.password ? formData.password : undefined,
      }
      onSubmit(updateData)
    } else {
      // Generate email from username for create
      const createData = {
        ...formData,
        email: `${formData.username}@local.host`,
      }
      onSubmit(createData)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>{mode === "create" ? "Create User" : "Edit User"}</DialogTitle>
          <DialogDescription>
            {mode === "create"
              ? "Add a new user to the system."
              : "Update user information. Leave password empty to keep current password."}
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="username">Username</Label>
            <div className="relative">
              <Input
                autoComplete="off"
                id="username"
                value={formData.username}
                onChange={(e) => setFormData({ ...formData, username: e.target.value })}
                placeholder="Enter username"
                disabled={isLoading}
                className={`pr-10 ${
                  usernameStatus === "available"
                    ? "border-green-500 focus-visible:ring-green-500"
                    : usernameStatus === "taken"
                    ? "border-red-500 focus-visible:ring-red-500"
                    : ""
                }`}
              />
              {usernameStatus === "checking" && (
                <div className="absolute right-3 top-1/2 -translate-y-1/2">
                  <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                </div>
              )}
              {usernameStatus === "available" && (
                <div className="absolute right-3 top-1/2 -translate-y-1/2">
                  <CheckCircle2 className="h-4 w-4 text-green-500" />
                </div>
              )}
              {usernameStatus === "taken" && (
                <div className="absolute right-3 top-1/2 -translate-y-1/2">
                  <XCircle className="h-4 w-4 text-red-500" />
                </div>
              )}
            </div>
            {usernameStatus === "available" && (
              <p className="text-sm text-green-600 flex items-center gap-1">
                <CheckCircle2 className="h-3.5 w-3.5" />
                Username is available
              </p>
            )}
            {usernameStatus === "taken" && (
              <p className="text-sm text-red-600 flex items-center gap-1">
                <XCircle className="h-3.5 w-3.5" />
                Username is already taken
              </p>
            )}
            {errors.username && <p className="text-sm text-red-600">{errors.username}</p>}
          </div>

          <div className="space-y-2">
            <Label htmlFor="password">
              Password {mode === "edit" && <span className="text-muted-foreground">(optional)</span>}
            </Label>
            <div className="relative">
              <Input
                id="password"
                type={showPassword ? "text" : "password"}
                value={formData.password}
                onChange={(e) => setFormData({ ...formData, password: e.target.value })}
                placeholder={mode === "edit" ? "Leave empty to keep current" : "Enter password"}
                disabled={isLoading}
                className="pr-10"
              />
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="absolute right-0 top-0 h-full px-3 py-2 hover:bg-transparent"
                onClick={() => setShowPassword(!showPassword)}
                disabled={isLoading}
              >
                {showPassword ? (
                  <EyeOff className="h-4 w-4 text-muted-foreground" />
                ) : (
                  <Eye className="h-4 w-4 text-muted-foreground" />
                )}
                <span className="sr-only">{showPassword ? "Hide password" : "Show password"}</span>
              </Button>
            </div>
            {errors.password && <p className="text-sm text-red-600">{errors.password}</p>}
          </div>

          <div className="space-y-2">
            <Label htmlFor="role">Role</Label>
            <Select
              value={formData.role}
              onValueChange={(value: "admin" | "user" | "viewer") =>
                setFormData({ ...formData, role: value })
              }
              disabled={isLoading}
            >
              <SelectTrigger>
                <SelectValue placeholder="Select role" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="admin">Admin - Full access</SelectItem>
                <SelectItem value="user">User - Standard access</SelectItem>
                <SelectItem value="viewer">Viewer - Read-only access</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={isLoading}>
              Cancel
            </Button>
            <Button type="submit" disabled={isLoading}>
              {isLoading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {mode === "create" ? "Creating..." : "Updating..."}
                </>
              ) : (
                <>{mode === "create" ? "Create User" : "Update User"}</>
              )}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
