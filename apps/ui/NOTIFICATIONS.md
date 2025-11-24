# Notification System

Sistem notifikasi real-time untuk NQRust-MicroVM UI yang menampilkan event dan status perubahan resources.

## Features

- ✅ Real-time notifications dengan badge counter
- ✅ Persistent storage menggunakan Zustand + localStorage
- ✅ Multiple notification types: info, success, warning, error
- ✅ Auto-link ke resource detail pages
- ✅ Mark as read/unread functionality
- ✅ Bulk actions (mark all read, clear all)
- ✅ Timestamp dengan relative time display
- ✅ Scroll area untuk banyak notifikasi
- ✅ Limit 50 notifikasi maksimal

## Components

### NotificationDropdown

Komponen dropdown yang ditampilkan di topbar untuk menampilkan semua notifikasi.

**Location:** `components/layout/notification-dropdown.tsx`

**Features:**
- Badge counter untuk unread notifications
- Dropdown dengan scroll area
- Notifikasi terkelompok dengan icon berdasarkan type
- Action buttons: mark as read, delete individual, mark all read, clear all
- Click notification untuk navigate ke resource detail page

### NotificationDemo

Komponen demo untuk testing notification system.

**Location:** `components/layout/notification-demo.tsx`

**Usage:**
```tsx
import { NotificationDemo } from "@/components/layout/notification-demo"

export default function Page() {
  return <NotificationDemo />
}
```

## Hooks

### useNotifications

Hook utility untuk trigger notifications dari anywhere dalam aplikasi.

**Location:** `lib/hooks/use-notifications.ts`

**Basic Usage:**
```tsx
import { useNotifications } from "@/lib/hooks/use-notifications"

function MyComponent() {
  const notifications = useNotifications()

  const handleCreateVM = async () => {
    try {
      const vm = await createVM(...)
      notifications.notifyVMCreated(vm.name, vm.id)
    } catch (error) {
      notifications.notifyVMError(vm.name, vm.id, error.message)
    }
  }

  return <Button onClick={handleCreateVM}>Create VM</Button>
}
```

## Available Notification Methods

### VM Notifications
- `notifyVMCreated(vmName, vmId)` - Success notification when VM is created
- `notifyVMStarted(vmName, vmId)` - Success notification when VM starts
- `notifyVMStopped(vmName, vmId)` - Info notification when VM stops
- `notifyVMDeleted(vmName)` - Info notification when VM is deleted
- `notifyVMError(vmName, vmId, error)` - Error notification for VM errors

### Function Notifications
- `notifyFunctionCreated(functionName, functionId)` - Success notification when function is created
- `notifyFunctionDeployed(functionName, functionId)` - Success notification when function is deployed
- `notifyFunctionInvoked(functionName, functionId)` - Success notification when function is invoked
- `notifyFunctionDeleted(functionName)` - Info notification when function is deleted
- `notifyFunctionError(functionName, functionId, error)` - Error notification for function errors

### Container Notifications
- `notifyContainerCreated(containerName, containerId)` - Success notification when container is created
- `notifyContainerStarted(containerName, containerId)` - Success notification when container starts
- `notifyContainerStopped(containerName, containerId)` - Info notification when container stops
- `notifyContainerDeleted(containerName)` - Info notification when container is deleted
- `notifyContainerError(containerName, containerId, error)` - Error notification for container errors

### Host Notifications
- `notifyHostConnected(hostName, hostId)` - Success notification when host connects
- `notifyHostDisconnected(hostName, hostId)` - Warning notification when host disconnects

### Generic Notifications
- `notify(type, title, message, actionUrl?)` - Generic notification with custom type

## Integration Examples

### Example 1: VM Creation
```tsx
import { useNotifications } from "@/lib/hooks/use-notifications"
import { useCreateVM } from "@/lib/queries"

function CreateVMForm() {
  const notifications = useNotifications()
  const createVM = useCreateVM()

  const handleSubmit = async (data) => {
    try {
      const vm = await createVM.mutateAsync(data)
      notifications.notifyVMCreated(vm.name, vm.id)
      router.push(`/vms/${vm.id}`)
    } catch (error) {
      notifications.notify('error', 'VM Creation Failed', error.message)
    }
  }

  return <form onSubmit={handleSubmit}>...</form>
}
```

### Example 2: VM State Changes
```tsx
import { useNotifications } from "@/lib/hooks/use-notifications"
import { useVmStatePatch } from "@/lib/queries"

function VMControls({ vm }) {
  const notifications = useNotifications()
  const vmStatePatch = useVmStatePatch()

  const handleStart = async () => {
    try {
      await vmStatePatch.mutateAsync({ id: vm.id, action: 'start' })
      notifications.notifyVMStarted(vm.name, vm.id)
    } catch (error) {
      notifications.notifyVMError(vm.name, vm.id, error.message)
    }
  }

  return <Button onClick={handleStart}>Start VM</Button>
}
```

### Example 3: Function Invocation
```tsx
import { useNotifications } from "@/lib/hooks/use-notifications"
import { useInvokeFunction } from "@/lib/queries"

function InvokeButton({ functionData }) {
  const notifications = useNotifications()
  const invokeFunction = useInvokeFunction()

  const handleInvoke = async () => {
    try {
      await invokeFunction.mutateAsync({
        fnId: functionData.id,
        payload: { event: {} }
      })
      notifications.notifyFunctionInvoked(functionData.name, functionData.id)
    } catch (error) {
      notifications.notifyFunctionError(functionData.name, functionData.id, error.message)
    }
  }

  return <Button onClick={handleInvoke}>Invoke</Button>
}
```

## Store

### useNotificationStore

Zustand store untuk managing notification state.

**Location:** `lib/stores/notification-store.ts`

**Methods:**
- `addNotification(notification)` - Add new notification
- `markAsRead(id)` - Mark notification as read
- `markAllAsRead()` - Mark all notifications as read
- `removeNotification(id)` - Remove single notification
- `clearAll()` - Clear all notifications
- `getUnreadCount()` - Get count of unread notifications

**Storage:**
Notifications are persisted to localStorage with key `notification-storage`.

## Types

### Notification Interface
```typescript
interface Notification {
  id: string
  type: 'info' | 'success' | 'warning' | 'error'
  title: string
  message: string
  timestamp: Date
  read: boolean
  actionUrl?: string
  resourceType?: 'vm' | 'function' | 'container' | 'host'
  resourceId?: string
}
```

## Testing

To test the notification system:

1. Add the demo component to any page:
```tsx
import { NotificationDemo } from "@/components/layout/notification-demo"

export default function TestPage() {
  return (
    <div className="p-6">
      <NotificationDemo />
    </div>
  )
}
```

2. Click the buttons to trigger different notification types
3. Check the bell icon in topbar for notification badge
4. Click the bell icon to see the notification dropdown

## Future Enhancements

- [ ] WebSocket integration for real-time server-sent notifications
- [ ] Notification preferences (enable/disable types)
- [ ] Sound notifications
- [ ] Desktop notifications (browser API)
- [ ] Email notifications for critical events
- [ ] Notification categories/filters
- [ ] Export notification history
