"use client"

import { Bell, Check, Trash2, CheckCheck, Info, AlertTriangle, AlertCircle, CheckCircle } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { ScrollArea } from "@/components/ui/scroll-area"
import { useNotificationStore } from "@/lib/stores/notification-store"
import { useDateFormat } from "@/lib/hooks/use-date-format"
import Link from "next/link"
import { cn } from "@/lib/utils"
import type { NotificationType } from "@/lib/types/notifications"

const getNotificationIcon = (type: NotificationType) => {
  switch (type) {
    case 'success':
      return <CheckCircle className="h-4 w-4 text-green-600" />
    case 'error':
      return <AlertCircle className="h-4 w-4 text-red-600" />
    case 'warning':
      return <AlertTriangle className="h-4 w-4 text-yellow-600" />
    case 'info':
    default:
      return <Info className="h-4 w-4 text-blue-600" />
  }
}

const getNotificationBgColor = (type: NotificationType) => {
  switch (type) {
    case 'success':
      return 'bg-green-500/10'
    case 'error':
      return 'bg-red-500/10'
    case 'warning':
      return 'bg-yellow-500/10'
    case 'info':
    default:
      return 'bg-blue-500/10'
  }
}

export function NotificationDropdown() {
  const { notifications, markAsRead, markAllAsRead, removeNotification, clearAll, getUnreadCount } = useNotificationStore()
  const dateFormat = useDateFormat()
  const unreadCount = getUnreadCount()

  const handleNotificationClick = (id: string, actionUrl?: string) => {
    markAsRead(id)
    if (actionUrl) {
      window.location.href = actionUrl
    }
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" className="relative h-9 w-9">
          <Bell className="h-5 w-5" />
          {unreadCount > 0 && (
            <Badge className="absolute -right-1 -top-1 h-5 min-w-[20px] rounded-full p-0 px-1 text-xs flex items-center justify-center bg-red-600 hover:bg-red-700">
              {unreadCount > 99 ? '99+' : unreadCount}
            </Badge>
          )}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-[380px] p-0">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-border p-4">
          <div className="flex items-center gap-2">
            <h3 className="font-semibold text-foreground">Notifications</h3>
            {unreadCount > 0 && (
              <Badge variant="secondary" className="text-xs">
                {unreadCount} new
              </Badge>
            )}
          </div>
          <div className="flex items-center gap-1">
            {unreadCount > 0 && (
              <Button
                variant="ghost"
                size="sm"
                onClick={markAllAsRead}
                title="Mark all as read"
                className="h-7 px-2 text-xs"
              >
                <CheckCheck className="h-3.5 w-3.5 mr-1" />
                Mark all
              </Button>
            )}
            {notifications.length > 0 && (
              <Button
                variant="ghost"
                size="sm"
                onClick={clearAll}
                title="Clear all"
                className="h-7 px-2 text-xs text-destructive hover:text-destructive"
              >
                <Trash2 className="h-3.5 w-3.5" />
              </Button>
            )}
          </div>
        </div>

        {/* Notification List */}
        <ScrollArea className="h-[400px]">
          {notifications.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 text-center px-4">
              <Bell className="h-12 w-12 text-muted-foreground/40 mb-3" />
              <p className="text-sm font-medium text-muted-foreground">No notifications</p>
              <p className="text-xs text-muted-foreground/70 mt-1">
                You&apos;re all caught up!
              </p>
            </div>
          ) : (
            <div className="divide-y divide-border">
              {notifications.map((notification) => (
                <div
                  key={notification.id}
                  className={cn(
                    "group relative p-4 transition-colors hover:bg-muted/50",
                    !notification.read && "bg-muted/30"
                  )}
                >
                  {/* Unread indicator */}
                  {!notification.read && (
                    <div className="absolute left-2 top-6 h-2 w-2 rounded-full bg-blue-600" />
                  )}

                  <div className="flex gap-3 pl-4">
                    {/* Icon */}
                    <div className={cn("rounded-lg p-2 h-fit", getNotificationBgColor(notification.type))}>
                      {getNotificationIcon(notification.type)}
                    </div>

                    {/* Content */}
                    <div className="flex-1 space-y-1 min-w-0">
                      <div
                        className={cn(
                          "cursor-pointer",
                          notification.actionUrl && "hover:underline"
                        )}
                        onClick={() => handleNotificationClick(notification.id, notification.actionUrl)}
                      >
                        <p className="text-sm font-medium text-foreground leading-tight">
                          {notification.title}
                        </p>
                        <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">
                          {notification.message}
                        </p>
                      </div>
                      <p className="text-xs text-muted-foreground/70">
                        {dateFormat.formatRelative(notification.timestamp)}
                      </p>
                    </div>

                    {/* Actions */}
                    <div className="flex items-start gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                      {!notification.read && (
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-7 w-7"
                          onClick={() => markAsRead(notification.id)}
                          title="Mark as read"
                        >
                          <Check className="h-3.5 w-3.5" />
                        </Button>
                      )}
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7 text-destructive hover:text-destructive"
                        onClick={() => removeNotification(notification.id)}
                        title="Remove"
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </Button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </ScrollArea>

        {/* Footer */}
        {notifications.length > 0 && (
          <div className="border-t border-border p-2">
            <Button
              variant="ghost"
              className="w-full text-xs text-muted-foreground hover:text-foreground"
              onClick={clearAll}
            >
              Clear all notifications
            </Button>
          </div>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
