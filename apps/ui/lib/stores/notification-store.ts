import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { Notification, NotificationStore } from '@/lib/types/notifications'

export const useNotificationStore = create<NotificationStore>()(
  persist(
    (set, get) => ({
      notifications: [],

      addNotification: (notification) => {
        const newNotification: Notification = {
          ...notification,
          id: `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
          timestamp: new Date(),
          read: false,
        }

        set((state) => ({
          notifications: [newNotification, ...state.notifications].slice(0, 50), // Keep max 50 notifications
        }))
      },

      markAsRead: (id) => {
        set((state) => ({
          notifications: state.notifications.map((n) =>
            n.id === id ? { ...n, read: true } : n
          ),
        }))
      },

      markAllAsRead: () => {
        set((state) => ({
          notifications: state.notifications.map((n) => ({ ...n, read: true })),
        }))
      },

      removeNotification: (id) => {
        set((state) => ({
          notifications: state.notifications.filter((n) => n.id !== id),
        }))
      },

      clearAll: () => {
        set({ notifications: [] })
      },

      getUnreadCount: () => {
        return get().notifications.filter((n) => !n.read).length
      },
    }),
    {
      name: 'notification-storage',
      partialize: (state) => ({ notifications: state.notifications }),
    }
  )
)
