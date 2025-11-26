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
import { Loader2 } from "lucide-react"
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
  }, [mode, user, open])

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
            <Input
              id="username"
              value={formData.username}
              onChange={(e) => setFormData({ ...formData, username: e.target.value })}
              placeholder="Enter username"
              disabled={isLoading}
            />
            {errors.username && <p className="text-sm text-red-600">{errors.username}</p>}
          </div>

          <div className="space-y-2">
            <Label htmlFor="password">
              Password {mode === "edit" && <span className="text-muted-foreground">(optional)</span>}
            </Label>
            <Input
              id="password"
              type="password"
              value={formData.password}
              onChange={(e) => setFormData({ ...formData, password: e.target.value })}
              placeholder={mode === "edit" ? "Leave empty to keep current" : "Enter password"}
              disabled={isLoading}
            />
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
