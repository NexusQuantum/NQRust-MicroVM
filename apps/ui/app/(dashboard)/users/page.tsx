"use client"

import { UserTable } from "@/components/user"
import { useUsers, useCreateUser, useUpdateUser, useDeleteUser } from "@/lib/queries"
import { Skeleton } from "@/components/ui/skeleton"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { AlertCircle, Users, Shield } from "lucide-react"
import type { CreateUserRequest, UpdateUserRequest } from "@/lib/types"
import { toast } from "sonner"

export default function UserPage() {
  const { data: users, isLoading, error } = useUsers()
  const createMutation = useCreateUser()
  const updateMutation = useUpdateUser()
  const deleteMutation = useDeleteUser()

  const handleCreateUser = (data: CreateUserRequest) => {
    createMutation.mutate(data, {
      onSuccess: () => {
        toast.success("User Created", {
          description: `User ${data.username} has been created successfully`,
        })
      },
      onError: (error: Error) => {
        // Check if error is from duplicate username
        const errorMessage = error.message.toLowerCase()
        if (errorMessage.includes("duplicate") || errorMessage.includes("already exists") || errorMessage.includes("unique")) {
          toast.error("Username Already Exists", {
            description: `Username "${data.username}" is already taken. Please choose a different username.`,
          })
        } else {
          toast.error("Create Failed", {
            description: `Failed to create user: ${error.message}`,
          })
        }
      },
    })
  }

  const handleUpdateUser = (id: string, data: UpdateUserRequest) => {
    updateMutation.mutate({ id, params: data }, {
      onSuccess: () => {
        toast.success("User Updated", {
          description: "User has been updated successfully",
        })
      },
      onError: (error: Error) => {
        const errorMessage = error.message.toLowerCase()
        if (errorMessage.includes("duplicate") || errorMessage.includes("already exists") || errorMessage.includes("unique")) {
          toast.error("Username Already Exists", {
            description: `Username "${data.username}" is already taken. Please choose a different username.`,
          })
        } else {
          toast.error("Update Failed", {
            description: `Failed to update user: ${error.message}`,
          })
        }
      },
    })
  }

  const handleDeleteUser = (id: string) => {
    deleteMutation.mutate(id, {
      onSuccess: () => {
        toast.success("User Deleted", {
          description: "User has been deleted successfully",
        })
      },
      onError: (error: Error) => {
        toast.error("Delete Failed", {
          description: `Failed to delete user: ${error.message}`,
        })
      },
    })
  }

  if (isLoading) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-48 w-full rounded-xl" />
        <Skeleton className="h-64 w-full" />
      </div>
    )
  }

  if (error) {
    return (
      <Alert variant="destructive">
        <AlertCircle className="h-4 w-4" />
        <AlertDescription>
          Failed to load users. Please try again later.
        </AlertDescription>
      </Alert>
    )
  }

  return (
    <div className="space-y-6">
      {/* Header Section */}
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-blue-50 to-blue-100/50 dark:from-blue-950/30 dark:to-blue-900/20 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <div className="flex items-center gap-3 mb-2">
              <div className="rounded-lg bg-blue-500/10 p-2">
                <Users className="h-8 w-8 text-blue-600 dark:text-blue-400" />
              </div>
              <h1 className="text-3xl font-bold text-foreground">User Management</h1>
            </div>
            <p className="mt-2 text-muted-foreground">
              Manage user accounts, roles, and permissions across your platform
            </p>
            <div className="mt-4 flex items-center gap-4">
              <div className="flex items-center gap-2">
                <Shield className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                <span className="text-sm font-medium">{users?.length || 0} Total Users</span>
              </div>
              <div className="flex items-center gap-2">
                <Users className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                <span className="text-sm font-medium">
                  {users?.filter(u => u.role === 'admin').length || 0} Admins
                </span>
              </div>
            </div>
          </div>
          <div className="hidden lg:block">
            <div className="flex h-48 w-48 items-center justify-center rounded-2xl bg-gradient-to-br from-blue-100 to-blue-200 dark:from-blue-900/30 dark:to-blue-800/30 shadow-lg">
              <Users className="h-24 w-24 text-blue-400 dark:text-blue-500" />
            </div>
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-blue-400/30 to-blue-600/30 blur-3xl" />
      </div>

      {/* User Table */}
      <UserTable
        users={users || []}
        onCreateUser={handleCreateUser}
        onUpdateUser={handleUpdateUser}
        onDeleteUser={handleDeleteUser}
        isCreating={createMutation.isPending}
        isUpdating={updateMutation.isPending}
        isDeleting={deleteMutation.isPending}
        isCreateSuccess={createMutation.isSuccess}
        isUpdateSuccess={updateMutation.isSuccess}
      />
    </div>
  )
}