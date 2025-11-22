import { useNotificationStore } from '@/lib/stores/notification-store'
import type { NotificationType } from '@/lib/types/notifications'

export function useNotifications() {
  const { addNotification } = useNotificationStore()

  return {
    // VM notifications
    notifyVMCreated: (vmName: string, vmId: string) => {
      addNotification({
        type: 'success',
        title: 'VM Created',
        message: `Virtual machine "${vmName}" has been created successfully`,
        actionUrl: `/vms/${vmId}`,
        resourceType: 'vm',
        resourceId: vmId,
      })
    },

    notifyVMStarted: (vmName: string, vmId: string) => {
      addNotification({
        type: 'success',
        title: 'VM Started',
        message: `Virtual machine "${vmName}" is now running`,
        actionUrl: `/vms/${vmId}`,
        resourceType: 'vm',
        resourceId: vmId,
      })
    },

    notifyVMStopped: (vmName: string, vmId: string) => {
      addNotification({
        type: 'info',
        title: 'VM Stopped',
        message: `Virtual machine "${vmName}" has been stopped`,
        actionUrl: `/vms/${vmId}`,
        resourceType: 'vm',
        resourceId: vmId,
      })
    },

    notifyVMDeleted: (vmName: string) => {
      addNotification({
        type: 'info',
        title: 'VM Deleted',
        message: `Virtual machine "${vmName}" has been deleted`,
        resourceType: 'vm',
      })
    },

    notifyVMError: (vmName: string, vmId: string, error: string) => {
      addNotification({
        type: 'error',
        title: 'VM Error',
        message: `Error on "${vmName}": ${error}`,
        actionUrl: `/vms/${vmId}`,
        resourceType: 'vm',
        resourceId: vmId,
      })
    },

    // Function notifications
    notifyFunctionCreated: (functionName: string, functionId: string) => {
      addNotification({
        type: 'success',
        title: 'Function Created',
        message: `Function "${functionName}" has been created successfully`,
        actionUrl: `/functions/${functionId}`,
        resourceType: 'function',
        resourceId: functionId,
      })
    },

    notifyFunctionDeployed: (functionName: string, functionId: string) => {
      addNotification({
        type: 'success',
        title: 'Function Deployed',
        message: `Function "${functionName}" is now ready to invoke`,
        actionUrl: `/functions/${functionId}`,
        resourceType: 'function',
        resourceId: functionId,
      })
    },

    notifyFunctionInvoked: (functionName: string, functionId: string) => {
      addNotification({
        type: 'success',
        title: 'Function Invoked',
        message: `Function "${functionName}" executed successfully`,
        actionUrl: `/functions/${functionId}?tab=logs`,
        resourceType: 'function',
        resourceId: functionId,
      })
    },

    notifyFunctionDeleted: (functionName: string) => {
      addNotification({
        type: 'info',
        title: 'Function Deleted',
        message: `Function "${functionName}" has been deleted`,
        resourceType: 'function',
      })
    },

    notifyFunctionError: (functionName: string, functionId: string, error: string) => {
      addNotification({
        type: 'error',
        title: 'Function Error',
        message: `Error on "${functionName}": ${error}`,
        actionUrl: `/functions/${functionId}?tab=logs`,
        resourceType: 'function',
        resourceId: functionId,
      })
    },

    // Container notifications
    notifyContainerCreated: (containerName: string, containerId: string) => {
      addNotification({
        type: 'success',
        title: 'Container Created',
        message: `Container "${containerName}" has been created successfully`,
        actionUrl: `/containers/${containerId}`,
        resourceType: 'container',
        resourceId: containerId,
      })
    },

    notifyContainerStarted: (containerName: string, containerId: string) => {
      addNotification({
        type: 'success',
        title: 'Container Started',
        message: `Container "${containerName}" is now running`,
        actionUrl: `/containers/${containerId}`,
        resourceType: 'container',
        resourceId: containerId,
      })
    },

    notifyContainerStopped: (containerName: string, containerId: string) => {
      addNotification({
        type: 'info',
        title: 'Container Stopped',
        message: `Container "${containerName}" has been stopped`,
        actionUrl: `/containers/${containerId}`,
        resourceType: 'container',
        resourceId: containerId,
      })
    },

    notifyContainerDeleted: (containerName: string) => {
      addNotification({
        type: 'info',
        title: 'Container Deleted',
        message: `Container "${containerName}" has been deleted`,
        resourceType: 'container',
      })
    },

    notifyContainerError: (containerName: string, containerId: string, error: string) => {
      addNotification({
        type: 'error',
        title: 'Container Error',
        message: `Error on "${containerName}": ${error}`,
        actionUrl: `/containers/${containerId}`,
        resourceType: 'container',
        resourceId: containerId,
      })
    },

    // Host notifications
    notifyHostConnected: (hostName: string, hostId: string) => {
      addNotification({
        type: 'success',
        title: 'Host Connected',
        message: `Host "${hostName}" is now connected`,
        actionUrl: `/hosts`,
        resourceType: 'host',
        resourceId: hostId,
      })
    },

    notifyHostDisconnected: (hostName: string, hostId: string) => {
      addNotification({
        type: 'warning',
        title: 'Host Disconnected',
        message: `Host "${hostName}" has disconnected`,
        actionUrl: `/hosts`,
        resourceType: 'host',
        resourceId: hostId,
      })
    },

    // Generic notifications
    notify: (type: NotificationType, title: string, message: string, actionUrl?: string) => {
      addNotification({
        type,
        title,
        message,
        actionUrl,
      })
    },
  }
}
