//! Host-side resource accounting helpers for shared FC + QEMU deployments.
//!
//! Both Firecracker and QEMU run inside `systemd-run --scope` units, which
//! gives the kernel a cgroup to enforce per-VM memory and CPU limits.
//! This module renders the appropriate `--property` arguments so the spawn
//! helpers can apply them uniformly across both VMM kinds.

/// Convert vCPU count + CPU weight to systemd cgroup properties.
///
/// `CPUWeight` is a fair-share weight (1..=10000, default 100). For predictable
/// behaviour on noisy hosts we set CPUQuota proportional to vcpu count so a
/// VM that asks for 2 vCPUs cannot starve out 4 vCPUs worth of other VMs:
/// `CPUQuota = vcpu * 100%`. With KVM accel, the vCPU threads still run on
/// host CPUs but the cgroup caps total CPU time when there's contention.
pub fn cpu_properties(vcpu: u32) -> Vec<String> {
    let quota = vcpu.max(1) * 100;
    vec![
        format!("CPUQuota={}%", quota),
        // Reasonable middle weight; ops can tune via env var if needed.
        "CPUWeight=100".to_string(),
    ]
}

/// Convert memory size (MiB) to systemd MemoryMax / MemorySwapMax properties.
///
/// We pin MemoryMax to the requested allocation so a runaway VMM can't OOM
/// the host. MemorySwapMax=0 ensures we never silently page guest memory to
/// swap (which would tank performance and confuse SLAs).
pub fn memory_properties(mem_mib: u32) -> Vec<String> {
    vec![
        format!("MemoryMax={}M", mem_mib),
        "MemorySwapMax=0".to_string(),
    ]
}

/// Full set of resource cgroup properties for a VM. Caller threads these
/// into systemd-run as repeated `--property=KEY=VALUE` arguments.
pub fn vm_properties(vcpu: u32, mem_mib: u32) -> Vec<String> {
    let mut props = Vec::with_capacity(8);
    props.extend(cpu_properties(vcpu));
    props.extend(memory_properties(mem_mib));
    props
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_properties_scale_with_vcpu() {
        let one = cpu_properties(1);
        assert!(one.contains(&"CPUQuota=100%".to_string()));
        let four = cpu_properties(4);
        assert!(four.contains(&"CPUQuota=400%".to_string()));
    }

    #[test]
    fn memory_properties_include_max_and_swap() {
        let p = memory_properties(512);
        assert!(p.contains(&"MemoryMax=512M".to_string()));
        assert!(p.contains(&"MemorySwapMax=0".to_string()));
    }

    #[test]
    fn vm_properties_combines() {
        let p = vm_properties(2, 1024);
        assert!(p.iter().any(|s| s == "CPUQuota=200%"));
        assert!(p.iter().any(|s| s == "MemoryMax=1024M"));
    }

    #[test]
    fn zero_vcpu_is_treated_as_one() {
        let p = cpu_properties(0);
        assert!(p.contains(&"CPUQuota=100%".to_string()));
    }
}
