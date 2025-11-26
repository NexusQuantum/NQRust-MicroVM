"use client"

import { useMemo, useState, useEffect } from "react"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Badge } from "@/components/ui/badge"
import { Pencil, Trash2, Search, UserPlus } from "lucide-react"
import {
  Pagination,
  PaginationContent,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from "@/components/ui/pagination"
import type { User } from "@/lib/types"
import { useDateFormat } from "@/lib/hooks/use-date-format"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"
import { useToast } from "@/hooks/use-toast"
import { UserFormDialog } from "./user-form-dialog"
import { useAuthStore } from "@/lib/auth/store"

interface UserTableProps {
  users: User[]
  onDeleteUser: (id: string) => void
  onCreateUser: (data: any) => void
  onUpdateUser: (id: string, data: any) => void
  isDeleting?: boolean
  isCreating?: boolean
  isUpdating?: boolean
  isCreateSuccess?: boolean
  isUpdateSuccess?: boolean
}

const ITEMS_PER_PAGE = 10

export function UserTable({
  users,
  onDeleteUser,
  onCreateUser,
  onUpdateUser,
  isDeleting = false,
  isCreating = false,
  isUpdating = false,
  isCreateSuccess = false,
  isUpdateSuccess = false
}: UserTableProps) {
  // ---- auth ----
  const { user: currentUser } = useAuthStore()
  const dateFormat = useDateFormat()

  // ---- filters/pagination ----
  const [searchQuery, setSearchQuery] = useState("")
  const [roleFilter, setRoleFilter] = useState<string>("all")
  const [currentPage, setCurrentPage] = useState(1)

  const filteredUsers = useMemo(() => {
    return users.filter((user) => {
      const matchesSearch = user.username.toLowerCase().includes(searchQuery.toLowerCase())
      const matchesRole = roleFilter === "all" || user.role === roleFilter
      return matchesSearch && matchesRole
    })
  }, [users, searchQuery, roleFilter])

  const totalPages = Math.ceil(filteredUsers.length / ITEMS_PER_PAGE)
  const startIndex = (currentPage - 1) * ITEMS_PER_PAGE
  const paginatedUsers = filteredUsers.slice(startIndex, startIndex + ITEMS_PER_PAGE)

  // ---- delete ----
  const { toast } = useToast()
  const [deleteDialog, setDeleteDialog] = useState<{ open: boolean; userId: string; username: string }>({
    open: false,
    userId: "",
    username: "",
  })

  const handleDelete = () => {
    if (deleteDialog.userId) {
      onDeleteUser(deleteDialog.userId)
      setDeleteDialog({ open: false, userId: "", username: "" })
    }
  }

  // ---- create/edit ----
  const [formDialog, setFormDialog] = useState<{ open: boolean; mode: "create" | "edit"; user?: User }>({
    open: false,
    mode: "create",
  })
  const [wasSubmitting, setWasSubmitting] = useState(false)

  const handleOpenCreate = () => {
    setFormDialog({ open: true, mode: "create" })
  }

  const handleOpenEdit = (user: User) => {
    setFormDialog({ open: true, mode: "edit", user })
  }

  const handleCloseForm = () => {
    setFormDialog({ open: false, mode: "create" })
    setWasSubmitting(false)
  }

  const handleSubmitForm = (data: any) => {
    setWasSubmitting(true)
    if (formDialog.mode === "create") {
      onCreateUser(data)
    } else if (formDialog.user) {
      onUpdateUser(formDialog.user.id, data)
    }
  }

  // Close dialog only when mutation succeeds
  useEffect(() => {
    if (wasSubmitting && (isCreateSuccess || isUpdateSuccess)) {
      // Mutation succeeded, close the dialog
      handleCloseForm()
    }
  }, [isCreateSuccess, isUpdateSuccess, wasSubmitting])

  // badge helper
  const getRoleBadge = (role: string) => {
    const colors = {
      admin: "bg-red-100 text-red-700 border-red-200",
      user: "bg-blue-100 text-blue-700 border-blue-200",
      viewer: "bg-gray-100 text-gray-700 border-gray-200",
    }
    const labels = {
      admin: "Admin",
      user: "User",
      viewer: "Viewer",
    }
    return (
      <Badge variant="outline" className={colors[role as keyof typeof colors]}>
        {labels[role as keyof typeof labels]}
      </Badge>
    )
  }

  return (
    <div className="space-y-4">
      {/* Header with Create Button */}
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">User Management</h2>
        <Button onClick={handleOpenCreate}>
          <UserPlus className="mr-2 h-4 w-4" />
          Create User
        </Button>
      </div>

      {/* Filters */}
      <div className="flex items-center gap-4">
        <div className="relative flex-1 min-w-0">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search users..."
            value={searchQuery}
            onChange={(e) => {
              setSearchQuery(e.target.value)
              setCurrentPage(1)
            }}
            className="pl-9"
          />
        </div>
        <Select
          value={roleFilter}
          onValueChange={(value) => {
            setRoleFilter(value)
            setCurrentPage(1)
          }}
        >
          <SelectTrigger className="w-40">
            <SelectValue placeholder="Role" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Roles</SelectItem>
            <SelectItem value="admin">Admin</SelectItem>
            <SelectItem value="user">User</SelectItem>
            <SelectItem value="viewer">Viewer</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* Table */}
      <div className="rounded-lg border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Username</TableHead>
              {/* <TableHead>Email</TableHead> */}
              <TableHead>Role</TableHead>
              <TableHead>Created</TableHead>
              <TableHead>Last Login</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {paginatedUsers.length === 0 ? (
              <TableRow>
                <TableCell colSpan={5} className="text-center py-8 text-muted-foreground">
                  No users found
                </TableCell>
              </TableRow>
            ) : (
              paginatedUsers.map((user) => (
                <TableRow key={user.id}>
                  <TableCell className="font-medium">
                    <div className="flex items-center gap-2">
                      {user.username}
                      {currentUser && user.id === currentUser.id && (
                        <Badge variant="secondary" className="bg-blue-100 text-blue-700 border-blue-200">
                          You
                        </Badge>
                      )}
                    </div>
                  </TableCell>
                  {/* <TableCell>{user.email}</TableCell> */}
                  <TableCell>{getRoleBadge(user.role)}</TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {dateFormat.formatRelative(user.created_at)}
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {user.last_login_at ? dateFormat.formatRelative(user.last_login_at) : "Never"}
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex justify-end gap-1">
                      <Button
                        variant="ghost"
                        size="icon"
                        title="Edit"
                        onClick={() => handleOpenEdit(user)}
                      >
                        <Pencil className="h-4 w-4" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        title={currentUser && user.id === currentUser.id ? "Cannot delete yourself" : "Delete"}
                        onClick={() => setDeleteDialog({ open: true, userId: user.id, username: user.username })}
                        disabled={currentUser && user.id === currentUser.id}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <Pagination>
          <PaginationContent>
            <PaginationItem>
              <PaginationPrevious
                size={undefined}
                onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
                className={currentPage === 1 ? "pointer-events-none opacity-50" : "cursor-pointer"}
              />
            </PaginationItem>
            {Array.from({ length: totalPages }, (_, i) => i + 1).map((page) => (
              <PaginationItem key={page}>
                <PaginationLink
                  size={undefined}
                  onClick={() => setCurrentPage(page)}
                  isActive={currentPage === page}
                  className="cursor-pointer"
                >
                  {page}
                </PaginationLink>
              </PaginationItem>
            ))}
            <PaginationItem>
              <PaginationNext
                size={undefined}
                onClick={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
                className={currentPage === totalPages ? "pointer-events-none opacity-50" : "cursor-pointer"}
              />
            </PaginationItem>
          </PaginationContent>
        </Pagination>
      )}

      {/* Delete dialog */}
      <ConfirmDialog
        open={deleteDialog.open}
        onOpenChange={(open) => setDeleteDialog({ ...deleteDialog, open })}
        title="Delete User"
        description={`Are you sure you want to delete ${deleteDialog.username}? This action cannot be undone.`}
        confirmText="Delete"
        onConfirm={handleDelete}
        variant="destructive"
      />

      {/* Create/Edit Form Dialog */}
      <UserFormDialog
        open={formDialog.open}
        onOpenChange={handleCloseForm}
        mode={formDialog.mode}
        user={formDialog.user}
        onSubmit={handleSubmitForm}
        isLoading={formDialog.mode === "create" ? isCreating : isUpdating}
      />
    </div>
  )
}
