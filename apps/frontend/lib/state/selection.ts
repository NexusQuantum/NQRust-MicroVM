import { create } from "zustand"
import { subscribeWithSelector } from "zustand/middleware"

interface VMSelectionState {
  selectedVMs: Set<string>
  isAllSelected: boolean
  selectVM: (vmId: string) => void
  deselectVM: (vmId: string) => void
  toggleVM: (vmId: string) => void
  selectAll: (vmIds: string[]) => void
  deselectAll: () => void
  toggleAll: (vmIds: string[]) => void
}

export const useVMSelectionStore = create<VMSelectionState>()(
  subscribeWithSelector((set, get) => ({
    selectedVMs: new Set(),
    isAllSelected: false,

    selectVM: (vmId) =>
      set((state) => ({
        selectedVMs: new Set(state.selectedVMs).add(vmId),
      })),

    deselectVM: (vmId) =>
      set((state) => {
        const newSelectedVMs = new Set(state.selectedVMs)
        newSelectedVMs.delete(vmId)
        return {
          selectedVMs: newSelectedVMs,
          isAllSelected: false,
        }
      }),

    toggleVM: (vmId) => {
      const { selectedVMs, selectVM, deselectVM } = get()
      if (selectedVMs.has(vmId)) {
        deselectVM(vmId)
      } else {
        selectVM(vmId)
      }
    },

    selectAll: (vmIds) =>
      set({
        selectedVMs: new Set(vmIds),
        isAllSelected: true,
      }),

    deselectAll: () =>
      set({
        selectedVMs: new Set(),
        isAllSelected: false,
      }),

    toggleAll: (vmIds) => {
      const { isAllSelected, selectAll, deselectAll } = get()
      if (isAllSelected) {
        deselectAll()
      } else {
        selectAll(vmIds)
      }
    },
  }))
)

// UI State for dashboard and other components
interface UIState {
  sidebarOpen: boolean
  setSidebarOpen: (open: boolean) => void
  currentView: "dashboard" | "vms" | "settings" | "health"
  setCurrentView: (view: UIState["currentView"]) => void
  chartsTimeWindow: "1m" | "5m" | "1h" | "6h" | "24h"
  setChartsTimeWindow: (window: UIState["chartsTimeWindow"]) => void
  drawerOpen: boolean
  setDrawerOpen: (open: boolean) => void
}

export const useUIStore = create<UIState>()(
  subscribeWithSelector((set) => ({
    sidebarOpen: true,
    setSidebarOpen: (open) => set({ sidebarOpen: open }),

    currentView: "dashboard",
    setCurrentView: (view) => set({ currentView: view }),

    chartsTimeWindow: "5m",
    setChartsTimeWindow: (window) => set({ chartsTimeWindow: window }),

    drawerOpen: false,
    setDrawerOpen: (open) => set({ drawerOpen: open }),
  }))
)

// Theme state (if not using next-themes)
interface ThemeState {
  theme: "light" | "dark" | "system"
  setTheme: (theme: ThemeState["theme"]) => void
}

export const useThemeStore = create<ThemeState>()((set) => ({
  theme: (process.env.NEXT_PUBLIC_BRAND_PRESET as ThemeState["theme"]) || "dark",
  setTheme: (theme) => set({ theme }),
}))

// Form state for VM creation wizard
interface VMCreationState {
  currentStep: number
  setCurrentStep: (step: number) => void
  formData: {
    metadata: {
      name: string
      description?: string
      tags: Record<string, string>
    }
    machine: {
      vcpu_count: number
      mem_size_mib: number
      smt: boolean
      cpu_template: string
    }
    boot: {
      kernel_image_path: string
      initrd_path?: string
      boot_args?: string
    }
    drives: any[]
    network_interfaces: any[]
  }
  updateFormData: (section: keyof VMCreationState["formData"], data: any) => void
  resetForm: () => void
}

const defaultFormData: VMCreationState["formData"] = {
  metadata: {
    name: "",
    description: "",
    tags: {},
  },
  machine: {
    vcpu_count: 1,
    mem_size_mib: 512,
    smt: false,
    cpu_template: "C3",
  },
  boot: {
    kernel_image_path: "",
    boot_args: "console=ttyS0 reboot=k panic=1 pci=off",
  },
  drives: [],
  network_interfaces: [],
}

export const useVMCreationStore = create<VMCreationState>()((set) => ({
  currentStep: 0,
  setCurrentStep: (step) => set({ currentStep: step }),

  formData: defaultFormData,

  updateFormData: (section, data) =>
    set((state) => ({
      formData: {
        ...state.formData,
        [section]: { ...state.formData[section], ...data },
      },
    })),

  resetForm: () =>
    set({
      currentStep: 0,
      formData: defaultFormData,
    }),
}))

// Export store selectors for easier usage
export const vmSelectionSelectors = {
  selectedCount: (state: VMSelectionState) => state.selectedVMs.size,
  hasSelection: (state: VMSelectionState) => state.selectedVMs.size > 0,
  isSelected: (vmId: string) => (state: VMSelectionState) =>
    state.selectedVMs.has(vmId),
}