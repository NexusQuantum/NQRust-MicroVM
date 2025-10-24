import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { facadeApi, parseFacadeError } from "./api";
import type { VmDrive, VmNic, Snapshot, Image, Template } from "@/lib/types";
import type { CreateVmReq } from "@/lib/types";
import { toast } from "sonner";

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
  templates: ["templates"] as const,
  template: (id: string) => ["templates", id] as const,

  // functions
  functions: ["functions"] as const,
  function: (id: string) => ["functions", id] as const,
};

// Query Functions
export function useFunctions() {
  return useQuery({
    queryKey: queryKeys.functions,
    queryFn: () => facadeApi.getFunctions(),
    staleTime: 30 * 1000,
  });
}

export function useFunction(id: string) {
  return useQuery({
    queryKey: queryKeys.function(id),
    queryFn: () => facadeApi.getFunction(id),
    staleTime: 30 * 1000,
    enabled: !!id,
  });
}

// VM Queries
export function useVMs() {
  return useQuery({
    queryKey: queryKeys.vms,
    queryFn: () => facadeApi.getVMs(),
    staleTime: 30 * 1000, // 30 seconds
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
    queryFn: () => facadeApi.getRegistryImages(),
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
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
