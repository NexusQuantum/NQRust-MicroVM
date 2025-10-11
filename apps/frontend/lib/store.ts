import { create } from "zustand"
import { persist } from "zustand/middleware"

interface UIState {
  // Sidebar state
  sidebarOpen: boolean
  setSidebarOpen: (open: boolean) => void

  // Theme
  theme: "light" | "dark" | "system"
  setTheme: (theme: "light" | "dark" | "system") => void

  // Dashboard filters
  dashboardFilters: {
    search: string
    state: string[]
    owner: string[]
    environment: string[]
    tags: Record<string, string>
  }
  setDashboardFilters: (filters: Partial<UIState["dashboardFilters"]>) => void

  // Selected VMs for batch operations
  selectedVMs: string[]
  setSelectedVMs: (vmIds: string[]) => void
  toggleVMSelection: (vmId: string) => void
  clearSelection: () => void

  // Metrics chart settings
  metricsTimeWindow: "1m" | "5m" | "1h" | "6h" | "24h"
  setMetricsTimeWindow: (window: "1m" | "5m" | "1h" | "6h" | "24h") => void

  // VM creation wizard
  wizardStep: number
  setWizardStep: (step: number) => void
  wizardData: Record<string, any>
  setWizardData: (data: Record<string, any>) => void
  clearWizardData: () => void
}

export const useUIStore = create<UIState>()(
  persist(
    (set, get) => ({
      // Sidebar
      sidebarOpen: true,
      setSidebarOpen: (open) => set({ sidebarOpen: open }),

      // Theme
      theme: "system",
      setTheme: (theme) => set({ theme }),

      // Dashboard filters
      dashboardFilters: {
        search: "",
        state: [],
        owner: [],
        environment: [],
        tags: {},
      },
      setDashboardFilters: (filters) =>
        set((state) => ({
          dashboardFilters: { ...state.dashboardFilters, ...filters },
        })),

      // VM selection
      selectedVMs: [],
      setSelectedVMs: (vmIds) => set({ selectedVMs: vmIds }),
      toggleVMSelection: (vmId) =>
        set((state) => ({
          selectedVMs: state.selectedVMs.includes(vmId)
            ? state.selectedVMs.filter((id) => id !== vmId)
            : [...state.selectedVMs, vmId],
        })),
      clearSelection: () => set({ selectedVMs: [] }),

      // Metrics
      metricsTimeWindow: "1h",
      setMetricsTimeWindow: (window) => set({ metricsTimeWindow: window }),

      // Wizard
      wizardStep: 0,
      setWizardStep: (step) => set({ wizardStep: step }),
      wizardData: {},
      setWizardData: (data) =>
        set((state) => ({
          wizardData: { ...state.wizardData, ...data },
        })),
      clearWizardData: () => set({ wizardData: {}, wizardStep: 0 }),
    }),
    {
      name: "nexusrust-ui-store",
      partialize: (state) => ({
        theme: state.theme,
        sidebarOpen: state.sidebarOpen,
        dashboardFilters: state.dashboardFilters,
        metricsTimeWindow: state.metricsTimeWindow,
      }),
    },
  ),
)
