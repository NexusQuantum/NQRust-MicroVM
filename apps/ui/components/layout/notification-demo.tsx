"use client"

import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { useNotifications } from "@/lib/hooks/use-notifications"

/**
 * Demo component showing how to use the notification system
 * This can be imported in any page to test notifications
 *
 * Usage in a page:
 * import { NotificationDemo } from "@/components/layout/notification-demo"
 *
 * Then add <NotificationDemo /> to your page
 */
export function NotificationDemo() {
  const notifications = useNotifications()

  return (
    <Card className="w-full max-w-2xl">
      <CardHeader>
        <CardTitle>Notification System Demo</CardTitle>
        <CardDescription>
          Click the buttons below to trigger different types of notifications
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* VM Notifications */}
        <div className="space-y-2">
          <h3 className="text-sm font-semibold">VM Notifications</h3>
          <div className="flex flex-wrap gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notifyVMCreated("test-vm-01", "vm-123")}
            >
              VM Created
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notifyVMStarted("test-vm-01", "vm-123")}
            >
              VM Started
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notifyVMStopped("test-vm-01", "vm-123")}
            >
              VM Stopped
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notifyVMDeleted("test-vm-01")}
            >
              VM Deleted
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notifyVMError("test-vm-01", "vm-123", "Out of memory")}
            >
              VM Error
            </Button>
          </div>
        </div>

        {/* Function Notifications */}
        <div className="space-y-2">
          <h3 className="text-sm font-semibold">Function Notifications</h3>
          <div className="flex flex-wrap gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notifyFunctionCreated("hello-world", "fn-456")}
            >
              Function Created
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notifyFunctionDeployed("hello-world", "fn-456")}
            >
              Function Deployed
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notifyFunctionInvoked("hello-world", "fn-456")}
            >
              Function Invoked
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notifyFunctionDeleted("hello-world")}
            >
              Function Deleted
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notifyFunctionError("hello-world", "fn-456", "Runtime error")}
            >
              Function Error
            </Button>
          </div>
        </div>

        {/* Container Notifications */}
        <div className="space-y-2">
          <h3 className="text-sm font-semibold">Container Notifications</h3>
          <div className="flex flex-wrap gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notifyContainerCreated("nginx-01", "cnt-789")}
            >
              Container Created
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notifyContainerStarted("nginx-01", "cnt-789")}
            >
              Container Started
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notifyContainerStopped("nginx-01", "cnt-789")}
            >
              Container Stopped
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notifyContainerDeleted("nginx-01")}
            >
              Container Deleted
            </Button>
          </div>
        </div>

        {/* Host Notifications */}
        <div className="space-y-2">
          <h3 className="text-sm font-semibold">Host Notifications</h3>
          <div className="flex flex-wrap gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notifyHostConnected("host-01", "host-123")}
            >
              Host Connected
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notifyHostDisconnected("host-01", "host-123")}
            >
              Host Disconnected
            </Button>
          </div>
        </div>

        {/* Generic Notifications */}
        <div className="space-y-2">
          <h3 className="text-sm font-semibold">Generic Notifications</h3>
          <div className="flex flex-wrap gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notify("info", "Info", "This is an informational message")}
            >
              Info
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notify("success", "Success", "Operation completed successfully")}
            >
              Success
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notify("warning", "Warning", "This is a warning message")}
            >
              Warning
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => notifications.notify("error", "Error", "An error has occurred")}
            >
              Error
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}
