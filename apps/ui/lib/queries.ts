import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { facadeApi } from "./api"
import type { CreateVmReq, CreateFunction, UpdateFunction, InvokeFunction, TestFunction, Image, UpdateTemplateReq } from "@/lib/types"
import { toast } from "sonner"
import { useNotificationStore } from "@/lib/stores/notification-store"

/**
 * Filter out internal system images that should never be shown to users.
 * These images are only used internally by the system to create functions and containers.
 *
 * Excludes:
 * - Runtime images: container-runtime, node-runtime, python-runtime
 * - Per-function/container rootfs copies in /functions/ or /containers/ directories
 * - Images with runtime-related names
 */
function filterInternalImages(images: Image[]): Image[] {
  return images.filter((img) => {
    // Exclude per-function/container rootfs copies (internal, not user-facing)
    // Functions create: /srv/images/functions/{vm-id}.ext4
    // Containers create: /srv/images/containers/{vm-id}.ext4
    if (img.host_path?.includes('/functions/') || img.host_path?.includes('/containers/')) {
      console.log('🔒 Filtering out internal copy:', img.name, img.host_path)
      return false
    }

    // Exclude system runtime images by project tag
    if (img.project && ['container-runtime', 'node-runtime', 'python-runtime', 'internal'].includes(img.project)) {
      console.log('🔒 Filtering out runtime by project:', img.name, img.project)
      return false
    }

    // Exclude system runtime images by name pattern (fallback if project not set)
    // Matches: container-runtime, node-runtime, python-runtime, alpine-docker, etc.
    const runtimeNamePatterns = [
      /runtime/i,           // Matches anything with "runtime"
      /alpine-docker/i,     // Alpine Docker base
      /function-base/i,     // Function base images
      /container-base/i,    // Container base images
    ]

    const isRuntimeImage = runtimeNamePatterns.some(pattern => pattern.test(img.name))
    if (isRuntimeImage) {
      console.log('🔒 Filtering out runtime by name:', img.name)
      return false
    }

    // Show all other images (user-created kernels, rootfs, docker images)
    return true
  })
}

// Query Keys
export const queryKeys = {
  vms: ["vms"] as const,
  vm: (id: string) => ["vms", id] as const,
  vmMetrics: (id: string) => ["vms", id, "metrics"] as const,
  vmDrives: (vmId: string) => ["vms", vmId, "drives"] as const,
  vmNics: (vmId: string) => ["vms", vmId, "nics"] as const,
  snapshots: (vmId: string) => ["vms", vmId, "snapshots"] as const,
  registryImages: ["registry", "images"] as const,
  registryVolumes: ["registry", "volumes"] as const,

  // images
  image: (id: string) => ["images", id] as const,

  // templates
  templates: ["templates"] as const,
  template: (id: string) => ["templates", id] as const,

  // docker hub
  dockerHubSearch: (query: string) => ["dockerhub", "search", query] as const,
  dockerHubTags: (imageName: string) => ["dockerhub", "tags", imageName] as const,

  // functions
  functions: ["functions"] as const,
  function: (id: string) => ["functions", id] as const,

  // containers
  containers: ["containers"] as const,
  container: (id: string) => ["containers", id] as const,
  containerLogs: (id: string) => ["containers", id, "logs"] as const,
  containerStats: (id: string) => ["containers", id, "stats"] as const,

  // hosts
  hosts: ["hosts"] as const,
  host: (id: string) => ["hosts", id] as const,

  // networks
  networks: ["networks"] as const,
  network: (id: string) => ["networks", id] as const,
  networkVms: (id: string) => ["networks", id, "vms"] as const,

  // volumes
  volumes: ["volumes"] as const,
  volume: (id: string) => ["volumes", id] as const,

  // users
  users: ["users"] as const,
  user: (id: string) => ["users", id] as const,

  // user preferences
  preferences: ["auth", "me", "preferences"] as const,
  profile: ["auth", "me", "profile"] as const,
}

// Function Query
// ! GET ALL
export function useFunctions(refetchInterval?: number) {
  return useQuery({
    queryKey: queryKeys.functions,
    queryFn: () => facadeApi.getFunctions(),
    staleTime: 30 * 1000,
    refetchInterval,
  });
}

// !Detail
export function useFunction(id: string) {
  return useQuery({
    queryKey: queryKeys.function(id),
    queryFn: () => facadeApi.getFunction(id),
    staleTime: 30 * 1000,
    enabled: !!id,
  });
}

// !DELETE
export function useDeleteFunction() {
  const queryClient = useQueryClient()
  
  return useMutation({
    mutationFn: (id: string) => facadeApi.deleteFunction(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.functions })
      toast.success("Function deleted")
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message)
        toast.error(e.error || "Delete failed", {description: e.suggestion || e.fault_message})
      } catch {
        toast.error('Failed to delete Function')
      }
    }
  })
}
  
// !CREATE
export function useCreateFunction() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({ name, runtime, handler, code, vcpu, memory_mb}: CreateFunction) =>
      facadeApi.createFunction({ name, runtime, handler, code, vcpu, memory_mb}),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.functions })
      toast.success('Function Created')

      // Add notification to notification store
      const { addNotification } = useNotificationStore.getState()
      addNotification({
        type: 'info',
        title: 'Function Created',
        message: `Function "${(data as any)?.name || 'New Function'}" is being created. You'll be notified when it's ready.`,
        actionUrl: `/functions/${(data as any)?.id}`,
        resourceType: 'function',
        resourceId: (data as any)?.id,
      })
    },
    onError: (error: Error) => {
      try {
        const errorData = JSON.parse(error.message)
        toast.error(errorData.error, {
          description: errorData.suggestion || errorData.fault_message,
        })
      } catch {
        toast.error("Failed to create VM")
      }
    },
  })
}

// !UPDATE
export function useUpdateFunction() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({ fnId, data }: { fnId: string, data: UpdateFunction }) => facadeApi.updateFunction(fnId, data),
    onSuccess: (_, { fnId }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.function(fnId) })
      toast.success("Function updated")
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message)
        toast.error(e.error || "Failed o update function", {
          description: e.suggestion || e.fault_message
        })
      } catch {
        toast.error("Failed to update function")
      }
    }
  })
}

// !Test
export function useTestFunction() {
  return useMutation({
    mutationFn: (params: TestFunction) =>
      facadeApi.testFunction(params),
    onSuccess: (data: any) => {
      toast.success("Test executed successfully", {
        description: `Duration: ${data.duration_ms}ms`,
      })
      return data
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message)
        toast.error(e.error || "Test failed", {
          description: e.suggestion || e.fault_message,
        })
      } catch {
        toast.error("Failed to execute test")
      }
    },
  })
}

// !Invoke
export function useInvokeFunction() {
  return useMutation({
    mutationFn: ({ fnId, payload }: { fnId: string, payload: InvokeFunction }) =>
      facadeApi.invokeFunction(fnId, payload),
    onSuccess: (data: any) => {
      toast.success("Function invoked successfully", {
        description: `Duration: ${data.duration_ms}ms`,
      })
      return data
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message)
        toast.error(e.error || "Invocation failed", {
          description: e.suggestion || e.fault_message,
        })
      } catch {
        toast.error("Failed to invoke function")
      }
    },
  })
}

export function useFunctionLogs(id: string, filters?: { status?: string; limit?: number }) {
  return useQuery({
    queryKey: [...queryKeys.function(id), "logs", filters],
    queryFn: () => facadeApi.getFunctionLogs(id, filters),
    enabled: !!id,
    staleTime: 5 * 1000,
  })
}

// VM Queries
/**
 * Get all VMs, optionally filtering out internal VMs used by functions/containers
 * @param includeInternal - If false, filters out VMs owned by functions/containers (default: false)
 * @param refetchInterval - Optional auto-refresh interval in milliseconds
 */
export function useVMs(includeInternal = false, refetchInterval?: number) {
  return useQuery({
    queryKey: [...queryKeys.vms, includeInternal] as const,
    queryFn: async () => {
      const vms = await facadeApi.getVMs()

      // If includeInternal is true, return all VMs
      if (includeInternal) {
        return vms
      }

      // Filter out VMs tagged as function or container VMs
      // Functions have tag: "type:function"
      // Containers have tag: "type:container"
      return vms.filter(vm => {
        const tags = vm.tags || []
        return !tags.includes("type:function") && !tags.includes("type:container")
      })
    },
    staleTime: 30 * 1000, // 30 seconds
    refetchInterval,
  });
}

export function useVM(id: string) {
  return useQuery({
    queryKey: queryKeys.vm(id),
    queryFn: () => facadeApi.getVM(id),
    enabled: !!id,
    staleTime: 10 * 1000, // 10 seconds
  });
}

export function useRegistryImages() {
  return useQuery({
    queryKey: queryKeys.registryImages,
    queryFn: async () => {
      const images = await facadeApi.getRegistryImages()
      // Filter out internal system images globally
      return filterInternalImages(images)
    },
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}

// Image Queries
export function useImage(id: string) {
  return useQuery({
    queryKey: queryKeys.image(id),
    queryFn: () => facadeApi.getImage(id),
    
  })
}

// Template Queries
export function useTemplates() {
  return useQuery({
    queryKey: queryKeys.templates,
    queryFn: () => facadeApi.getTemplates(),
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}

export function useTemplate(id: string) {
  return useQuery({
    queryKey: queryKeys.template(id),
    queryFn: () => facadeApi.getTemplate(id),
    enabled: !!id,
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}

export function useDeleteTemplate() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => facadeApi.deleteTemplate(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.templates });
      // Toast will be shown after redirect to /templates
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to delete template", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to delete template");
      }
    },
  });
}

export function useUpdateTemplate() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateTemplateReq }) =>
      facadeApi.updateTemplate(id, data),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.template(id) });
      queryClient.invalidateQueries({ queryKey: queryKeys.templates });
      // Toast will be shown after redirect to /templates
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to update template", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to update template");
      }
    },
  });
}

export function useInstantiateTemplate() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) =>
      facadeApi.instantiateTemplate(id, { name }),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.vms });
      toast.success("VM created from template", {
        description: `VM "${data.id}" has been created successfully`,
      });
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to instantiate template", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to instantiate template");
      }
    },
  });
}

export function useImportRegistryImage() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (params: {
      type: "kernel" | "rootfs" | "data";
      name?: string;
      path?: string;
      url?: string;
    }) => facadeApi.importRegistryImage(params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.registryImages });
      toast.success("Image imported");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Import failed", { description: e.message });
      } catch {
        toast.error("Import failed");
      }
    },
  });
}

export function useCreateRegistryVolume() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (params: {
      name: string;
      size_bytes: number;
      type?: "rootfs" | "data";
    }) => facadeApi.createRegistryVolume(params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.registryImages });
      toast.success("Volume created");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Create failed", { description: e.message });
      } catch {
        toast.error("Create failed");
      }
    },
  });
}

export function useDeleteRegistryItem() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (path: string) => facadeApi.deleteRegistryItem(path),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.registryImages });
      toast.success("Item deleted");
    },
    onError: () => toast.error("Delete failed"),
  });
}

export function useRenameRegistryItem() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (p: { path: string; new_name: string }) =>
      facadeApi.renameRegistryItem(p.path, p.new_name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.registryImages });
      toast.success("Item renamed");
    },
    onError: () => toast.error("Rename not supported in current backend"),
  });
}

export function useUploadRegistryFile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (params: { type: "kernel" | "rootfs"; file: File }) => {
      // Upload not implemented in new backend yet
      throw new Error("File upload not implemented in current backend");
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.registryImages });
      toast.success("File uploaded");
    },
    onError: () => toast.error("Upload not supported in current backend"),
  });
}

// Docker Hub Queries
export function useSearchDockerHub(query: string, enabled = true) {
  return useQuery({
    queryKey: queryKeys.dockerHubSearch(query),
    queryFn: () => facadeApi.searchDockerHub(query, 25),
    enabled: enabled && query.length > 0,
    staleTime: 10 * 60 * 1000, // 10 minutes
  });
}

export function useDockerImageTags(imageName: string, enabled = true) {
  return useQuery({
    queryKey: queryKeys.dockerHubTags(imageName),
    queryFn: () => facadeApi.getDockerImageTags(imageName),
    enabled: enabled && imageName.length > 0,
    staleTime: 10 * 60 * 1000, // 10 minutes
  });
}

export function useDownloadDockerImage() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (params: { image: string; registry_auth?: { username: string; password: string; server_address?: string } }) =>
      facadeApi.downloadDockerImage(params),
    onSuccess: async () => {
      // Invalidate and immediately refetch the images list
      await queryClient.invalidateQueries({ queryKey: queryKeys.registryImages });
      await queryClient.refetchQueries({ queryKey: queryKeys.registryImages });
      toast.success("Docker image downloaded and cached", {
        description: "The image is now available in your cached Docker images",
      });
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Download failed", { 
          description: e.fault_message || e.message || "Please check the logs for details" 
        });
      } catch {
        toast.error("Download failed", { description: error.message || "Please check the logs for details" });
      }
    },
  });
}

export function useUploadImage() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (params: { file: File; kind: "docker" | "kernel" | "rootfs"; name?: string; project?: string }) =>
      facadeApi.uploadImage(params.file, params.kind, params.name, params.project),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.registryImages });
      toast.success("Image uploaded successfully");
    },
    onError: (error: Error) => {
      toast.error("Upload failed", { description: error.message });
    },
  });
}

// VM Mutations
export function useCreateVM() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (config: CreateVmReq) => facadeApi.createVM(config),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.vms });
      toast.success("VM created successfully");
    },
    onError: (error: Error) => {
      try {
        const errorData = JSON.parse(error.message);
        toast.error(errorData.error, {
          description: errorData.suggestion || errorData.fault_message,
        });
      } catch {
        toast.error("Failed to create VM");
      }
    },
  });
}

export function useInitializeVM() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => facadeApi.initializeVM(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.vm(id) });
      toast.success("VM initialized successfully");
    },
    onError: (error: Error) => {
      try {
        const errorData = JSON.parse(error.message);
        toast.error(errorData.error, {
          description: errorData.suggestion || errorData.fault_message,
        });
      } catch {
        toast.error("Failed to initialize VM");
      }
    },
  });
}

// Snapshots
export function useSnapshots(vmId: string) {
  return useQuery({
    queryKey: queryKeys.snapshots(vmId),
    queryFn: () => facadeApi.getVMSnapshots(vmId),
    enabled: !!vmId,
    staleTime: 30 * 1000, // 30 seconds
  });
}

export function useCreateSnapshot() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      vmId,
      snapshot_path,
      mem_file_path,
      snapshot_type,
      version,
    }: {
      vmId: string;
      snapshot_path: string;
      mem_file_path: string;
      snapshot_type?: "Full" | "Diff";
      version?: string;
    }) =>
      facadeApi.createSnapshot(vmId, {
        snapshot_path,
        mem_file_path,
        snapshot_type,
        version,
      }),
    onSuccess: (_, { vmId }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.snapshots(vmId) });
      queryClient.invalidateQueries({ queryKey: queryKeys.vm(vmId) });
    },
    onError: (error: Error) => {
      try {
        const errorData = JSON.parse(error.message);
        toast.error(errorData.error, {
          description: errorData.suggestion || errorData.fault_message,
        });
      } catch {
        toast.error("Failed to create snapshot");
      }
    },
  });
}

export function useRestoreSnapshot() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ vmId, snapshotId }: { vmId: string; snapshotId: string }) =>
      facadeApi.restoreSnapshot(vmId, {
        snapshot_path: snapshotId,
        mem_file_path: "",
      }),
    onSuccess: (_, { vmId }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.vm(vmId) });
      queryClient.invalidateQueries({ queryKey: queryKeys.vms });
    },
    onError: (error: Error) => {
      try {
        const errorData = JSON.parse(error.message);
        if (errorData.status === 409) {
          toast.error("Cannot restore snapshot", {
            description:
              errorData.fault_message ||
              "VM must be stopped to restore snapshot",
          });
        } else {
          toast.error(errorData.error, {
            description: errorData.suggestion || errorData.fault_message,
          });
        }
      } catch {
        toast.error("Failed to restore snapshot");
      }
    },
  });
}

export function useDeleteSnapshot() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ vmId, snapshotId }: { vmId: string; snapshotId: string }) =>
      facadeApi.deleteVMSnapshot(vmId, snapshotId),
    onSuccess: (_, { vmId }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.snapshots(vmId) });
    },
    onError: (error: Error) => {
      try {
        const errorData = JSON.parse(error.message);
        toast.error(errorData.error, {
          description: errorData.suggestion || errorData.fault_message,
        });
      } catch {
        toast.error("Failed to delete snapshot");
      }
    },
  });
}

// Facade VM State Patch
export type VmStateAction =
  | "start"
  | "stop"
  | "pause"
  | "resume"
  | "flush_metrics"
  | "ctrl_alt_del";

export function useVmStatePatch() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, action }: { id: string; action: VmStateAction }) =>
      facadeApi.updateVMState(id, action),
    onSuccess: (_, { id, action }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.vm(id) });
      queryClient.invalidateQueries({ queryKey: queryKeys.vms });
      switch (action) {
        case "start":
          toast.success("VM started");
          break;
        case "pause":
          toast.success("VM paused");
          break;
        case "resume":
          toast.success("VM resumed");
          break;
        case "stop":
          toast.success("Shutdown signal sent");
          break;
        case "flush_metrics":
          toast.success("Metrics flush requested");
          break;
        case "ctrl_alt_del":
          toast.success("Ctrl+Alt+Del signal sent");
          break;
      }
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Action failed", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Action failed");
      }
    },
  });
}

// VM Delete
export function useDeleteVM() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => facadeApi.deleteVM(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.vms });
      toast.success("VM deleted");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Delete failed", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to delete VM");
      }
    },
  });
}

// Drive Management Queries and Mutations
export function useVMDrives(vmId: string) {
  return useQuery({
    queryKey: queryKeys.vmDrives(vmId),
    queryFn: () => facadeApi.getVMDrives(vmId),
    enabled: !!vmId,
    staleTime: 30 * 1000, // 30 seconds
  });
}

export function useCreateVMDrive() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ vmId, drive }: { vmId: string; drive: any }) =>
      facadeApi.createVMDrive(vmId, drive),
    onSuccess: (_, { vmId }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.vmDrives(vmId) });
      queryClient.invalidateQueries({ queryKey: queryKeys.vm(vmId) });
      toast.success("Drive created");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to create drive", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to create drive");
      }
    },
  });
}

export function useUpdateVMDrive() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      vmId,
      driveId,
      drive,
    }: {
      vmId: string;
      driveId: string;
      drive: any;
    }) => facadeApi.updateVMDrive(vmId, driveId, drive),
    onSuccess: (_, { vmId }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.vmDrives(vmId) });
      queryClient.invalidateQueries({ queryKey: queryKeys.vm(vmId) });
      toast.success("Drive updated");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to update drive", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to update drive");
      }
    },
  });
}

export function useDeleteVMDrive() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ vmId, driveId }: { vmId: string; driveId: string }) =>
      facadeApi.deleteVMDrive(vmId, driveId),
    onSuccess: (_, { vmId }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.vmDrives(vmId) });
      queryClient.invalidateQueries({ queryKey: queryKeys.vm(vmId) });
      toast.success("Drive deleted");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to delete drive", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to delete drive");
      }
    },
  });
}

// Network Interface Management Queries and Mutations
export function useVMNics(vmId: string) {
  return useQuery({
    queryKey: queryKeys.vmNics(vmId),
    queryFn: () => facadeApi.getVMNics(vmId),
    enabled: !!vmId,
    staleTime: 30 * 1000, // 30 seconds
  });
}

export function useCreateVMNic() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ vmId, nic }: { vmId: string; nic: any }) =>
      facadeApi.createVMNic(vmId, nic),
    onSuccess: (_, { vmId }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.vmNics(vmId) });
      queryClient.invalidateQueries({ queryKey: queryKeys.vm(vmId) });
      toast.success("Network interface created");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to create NIC", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to create NIC");
      }
    },
  });
}

export function useUpdateVMNic() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      vmId,
      nicId,
      nic,
    }: {
      vmId: string;
      nicId: string;
      nic: any;
    }) => facadeApi.updateVMNic(vmId, nicId, nic),
    onSuccess: (_, { vmId }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.vmNics(vmId) });
      queryClient.invalidateQueries({ queryKey: queryKeys.vm(vmId) });
      toast.success("Network interface updated");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to update NIC", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to update NIC");
      }
    },
  });
}

export function useDeleteVMNic() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ vmId, nicId }: { vmId: string; nicId: string }) =>
      facadeApi.deleteVMNic(vmId, nicId),
    onSuccess: (_, { vmId }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.vmNics(vmId) });
      queryClient.invalidateQueries({ queryKey: queryKeys.vm(vmId) });
      toast.success("Network interface deleted");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to delete NIC", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to delete NIC");
      }
    },
  });
}

// Container Queries and Mutations
export function useContainers(refetchInterval?: number, filters?: { state?: string; host_id?: string }) {
  return useQuery({
    queryKey: [...queryKeys.containers, filters],
    queryFn: () => facadeApi.getContainers(filters),
    staleTime: 30 * 1000,
    refetchInterval,
  });
}

export function useContainer(id: string) {
  return useQuery({
    queryKey: queryKeys.container(id),
    queryFn: () => facadeApi.getContainer(id),
    enabled: !!id,
    staleTime: 5 * 1000, // 5 seconds for faster updates during provisioning
  });
}

export function useCreateContainer() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: import("@/lib/types").CreateContainerReq) =>
      facadeApi.createContainer(params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.containers });
      toast.success("Container created", {
        description: "Container is being provisioned...",
      });
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to create container", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to create container");
      }
    },
  });
}

export function useUpdateContainer() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, params }: { id: string; params: import("@/lib/types").UpdateContainerReq }) =>
      facadeApi.updateContainer(id, params),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.container(id) });
      queryClient.invalidateQueries({ queryKey: queryKeys.containers });
      toast.success("Container updated");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to update container", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to update container");
      }
    },
  });
}

export function useDeleteContainer() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => facadeApi.deleteContainer(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.containers });
      toast.success("Container deleted");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to delete container", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to delete container");
      }
    },
  });
}

export function useStartContainer() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => facadeApi.startContainer(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.container(id) });
      queryClient.invalidateQueries({ queryKey: queryKeys.containers });
      toast.success("Container started");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to start container", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to start container");
      }
    },
  });
}

export function useStopContainer() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => facadeApi.stopContainer(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.container(id) });
      queryClient.invalidateQueries({ queryKey: queryKeys.containers });
      toast.success("Container stopped");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to stop container", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to stop container");
      }
    },
  });
}

export function useRestartContainer() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => facadeApi.restartContainer(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.container(id) });
      queryClient.invalidateQueries({ queryKey: queryKeys.containers });
      toast.success("Container restarted");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to restart container", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to restart container");
      }
    },
  });
}

export function usePauseContainer() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => facadeApi.pauseContainer(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.container(id) });
      queryClient.invalidateQueries({ queryKey: queryKeys.containers });
      toast.success("Container paused");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to pause container", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to pause container");
      }
    },
  });
}

export function useResumeContainer() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => facadeApi.resumeContainer(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.container(id) });
      queryClient.invalidateQueries({ queryKey: queryKeys.containers });
      toast.success("Container resumed");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to resume container", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to resume container");
      }
    },
  });
}

export function useContainerLogs(id: string, tail?: number) {
  return useQuery({
    queryKey: [...queryKeys.containerLogs(id), tail],
    queryFn: () => facadeApi.getContainerLogs(id, tail),
    enabled: !!id,
    staleTime: 5 * 1000,
  });
}

export function useContainerStats(id: string) {
  return useQuery({
    queryKey: queryKeys.containerStats(id),
    queryFn: () => facadeApi.getContainerStats(id),
    enabled: !!id,
    staleTime: 5 * 1000,
    refetchInterval: 10 * 1000, // Refetch every 10 seconds for live stats
  });
}

// ==============
// Host Management Queries
// ==============

export function useHosts() {
  return useQuery({
    queryKey: queryKeys.hosts,
    queryFn: () => facadeApi.getHosts(),
    staleTime: 30 * 1000,
  });
}

export function useHost(id: string) {
  return useQuery({
    queryKey: queryKeys.host(id),
    queryFn: () => facadeApi.getHost(id),
    enabled: !!id,
    staleTime: 30 * 1000,
  });
}

export function useDeleteHost() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => facadeApi.deleteHost(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.hosts });
      toast.success("Host deleted successfully");
    },
    onError: (error: any) => {
      if (error?.response?.status === 400) {
        toast.error("Cannot delete alive host. Only dead hosts (offline for more than 30 seconds) can be deleted.");
      } else {
        toast.error(error?.error || "Failed to delete host");
      }
    },
  });
}

// ==============
// Network Management Queries
// ==============

export function useNetworks() {
  return useQuery({
    queryKey: queryKeys.networks,
    queryFn: () => facadeApi.getNetworks(),
    staleTime: 30 * 1000,
  });
}

export function useNetwork(id: string) {
  return useQuery({
    queryKey: queryKeys.network(id),
    queryFn: () => facadeApi.getNetwork(id),
    enabled: !!id,
    staleTime: 30 * 1000,
  });
}

export function useNetworkVms(id: string) {
  return useQuery({
    queryKey: queryKeys.networkVms(id),
    queryFn: () => facadeApi.getNetworkVms(id),
    enabled: !!id,
    staleTime: 30 * 1000,
  });
}

export function useUpdateNetwork() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, params }: { id: string; params: any }) =>
      facadeApi.updateNetwork(id, params),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.network(id) });
      queryClient.invalidateQueries({ queryKey: queryKeys.networks });
      toast.success("Network updated successfully");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to update network", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to update network");
      }
    },
  });
}

export function useCreateNetwork() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (network: any) => facadeApi.createNetwork(network),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.networks });
      toast.success("Network created successfully");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to create network", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to create network");
      }
    },
  });
}

export function useDeleteNetwork() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => facadeApi.deleteNetwork(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.networks });
      toast.success("Network deleted successfully");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to delete network", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to delete network");
      }
    },
  });
}

// ==============
// Volume Management Queries
// ==============

export function useVolumes() {
  return useQuery({
    queryKey: queryKeys.volumes,
    queryFn: () => facadeApi.getVolumes(),
    staleTime: 30 * 1000,
  });
}

export function useVolume(id: string) {
  return useQuery({
    queryKey: queryKeys.volume(id),
    queryFn: () => facadeApi.getVolume(id),
    enabled: !!id,
    staleTime: 30 * 1000,
  });
}

export function useCreateVolume() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: facadeApi.createVolume.bind(facadeApi),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.volumes });
      toast.success("Volume created successfully");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to create volume", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to create volume");
      }
    },
  });
}

export function useAttachVolume() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, params }: { id: string; params: any }) =>
      facadeApi.attachVolume(id, params),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.volume(id) });
      queryClient.invalidateQueries({ queryKey: queryKeys.volumes });
      toast.success("Volume attached successfully");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to attach volume", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to attach volume");
      }
    },
  });
}

export function useDetachVolume() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, params }: { id: string; params: any }) =>
      facadeApi.detachVolume(id, params),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.volume(id) });
      queryClient.invalidateQueries({ queryKey: queryKeys.volumes });
      toast.success("Volume detached successfully");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to detach volume", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to detach volume");
      }
    },
  });
}

export function useDeleteVolume() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => facadeApi.deleteVolume(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.volumes });
      toast.success("Volume deleted successfully");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to delete volume", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to delete volume");
      }
    },
  });
}

// ======================
// User Management Queries
// ======================

export function useUsers() {
  return useQuery({
    queryKey: queryKeys.users,
    queryFn: () => facadeApi.getUsers(),
    staleTime: 30 * 1000,
  });
}

export function useUser(id: string) {
  return useQuery({
    queryKey: queryKeys.user(id),
    queryFn: () => facadeApi.getUser(id),
    enabled: !!id,
    staleTime: 30 * 1000,
  });
}

export function useCreateUser() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: import("@/lib/types").CreateUserRequest) =>
      facadeApi.createUser(params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.users });
      toast.success("User created successfully");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to create user", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to create user");
      }
    },
  });
}

export function useUpdateUser() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, params }: { id: string; params: import("@/lib/types").UpdateUserRequest }) =>
      facadeApi.updateUser(id, params),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.user(id) });
      queryClient.invalidateQueries({ queryKey: queryKeys.users });
      toast.success("User updated successfully");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to update user", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to update user");
      }
    },
  });
}

export function useDeleteUser() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => facadeApi.deleteUser(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.users });
      toast.success("User deleted successfully");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to delete user", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to delete user");
      }
    },
  });
}

// ======================
// User Preferences & Profile Queries
// ======================

export function usePreferences() {
  return useQuery({
    queryKey: queryKeys.preferences,
    queryFn: () => facadeApi.getPreferences(),
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}

export function useUpdatePreferences() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: import("@/lib/types").UpdatePreferencesRequest) =>
      facadeApi.updatePreferences(params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.preferences });
      toast.success("Preferences updated successfully");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to update preferences", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to update preferences");
      }
    },
  });
}

export function useProfile() {
  return useQuery({
    queryKey: queryKeys.profile,
    queryFn: () => facadeApi.getProfile(),
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}

export function useUpdateProfile() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: import("@/lib/types").UpdateProfileRequest) =>
      facadeApi.updateProfile(params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.profile });
      toast.success("Profile updated successfully");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to update profile", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to update profile");
      }
    },
  });
}

export function useChangePassword() {
  return useMutation({
    mutationFn: (params: import("@/lib/types").ChangePasswordRequest) =>
      facadeApi.changePassword(params),
    onSuccess: () => {
      toast.success("Password changed successfully");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to change password", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to change password");
      }
    },
  });
}

export function useUploadAvatar() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (file: File) => facadeApi.uploadAvatar(file),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.profile });
      toast.success("Avatar uploaded successfully");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to upload avatar", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to upload avatar");
      }
    },
  });
}

export function useDeleteAvatar() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => facadeApi.deleteAvatar(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.profile });
      toast.success("Avatar deleted successfully");
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message);
        toast.error(e.error || "Failed to delete avatar", {
          description: e.suggestion || e.fault_message,
        });
      } catch {
        toast.error("Failed to delete avatar");
      }
    },
  });
}
