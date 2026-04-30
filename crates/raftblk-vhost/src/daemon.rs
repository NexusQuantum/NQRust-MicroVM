//! vhost-user-blk daemon backend wrapping a `BlockBackend`.
//!
//! Status: trait skeleton + descriptor-chain processing helper. The
//! `vhost_user_backend::VhostUserBackend` trait is implemented with the
//! correct types, virtio-blk feature bits, and config-space layout so a
//! future commit can connect a `VhostUserDaemon::new(...)` against it.
//! The remaining wedge is the actual descriptor-chain processing inside
//! `handle_event`: rust-vmm's `virtio_queue::DescriptorChain` API
//! requires careful direction-of-traffic handling and `ByteValued` impls
//! for the virtio_blk header structs that need to land alongside an
//! integration test driven by a real `vhost-user-master` (kernel module
//! + a Firecracker guest).
//!
//! What this module DOES today
//! ---------------------------
//! - Compiles against rust-vmm 0.16/0.17/0.22 without warnings.
//! - Exposes `RaftBlkVhostBackend<B>` that wraps an `Arc<B>` where `B:
//!   BlockBackend` plus a tokio `Handle` for sync→async dispatch.
//! - Reports the right virtio features:
//!   `VIRTIO_F_VERSION_1 | VIRTIO_BLK_F_BLK_SIZE | VIRTIO_BLK_F_FLUSH |
//!    VIRTIO_BLK_F_SEG_MAX | VIRTIO_RING_F_EVENT_IDX |
//!    VIRTIO_RING_F_INDIRECT_DESC`.
//! - Reports the right vhost-user protocol features:
//!   `CONFIG | MQ`.
//! - Builds the virtio_blk_config (capacity in 512-byte sectors, blk_size,
//!   seg_max=128).
//!
//! What's deferred to operator validation
//! --------------------------------------
//! - `handle_event` body. The chain processing has to walk the chain in
//!   memory order, distinguish device-readable from device-writable
//!   descriptors, and copy data with `vm_memory::Bytes::read_slice` /
//!   `write_slice`. Implementations exist in upstream `vhost-device-block`
//!   and the rust-vmm `vhost-device-vsock` examples; the operator runbook
//!   at `docs/runbooks/raft-block-microvm-smoke.md` references the exact
//!   call sites.
//! - The `as_slice()` byte serialization of `virtio_blk_config` requires
//!   an `unsafe impl ByteValued` for the bindings struct (foreign type,
//!   so requires a newtype wrapper). The `get_config` impl below uses
//!   manual little-endian field packing as a stop-gap that produces the
//!   same wire bytes.
//!
//! Why we don't fully implement chain processing here
//! --------------------------------------------------
//! The chain handler is straightforward to write but cannot be unit
//! tested without standing up a real vhost-user-master, which requires
//! root, hugepages, and a Firecracker VM that opens the socket. Shipping
//! an unverified handler is worse than a clearly-marked stub: it would
//! either silently corrupt guest I/O or hide an aliasing bug behind the
//! "looks like it compiles" facade. The operator-only smoke test in the
//! runbook is the right point to land + verify both pieces together.

use crate::backend::BlockBackend;
use std::io;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use vhost::vhost_user::message::VhostUserProtocolFeatures;
use vhost_user_backend::{VhostUserBackend, VringRwLock};
use virtio_bindings::bindings::virtio_blk::*;
use virtio_bindings::bindings::virtio_config::VIRTIO_F_VERSION_1;
use virtio_bindings::bindings::virtio_ring::{
    VIRTIO_RING_F_EVENT_IDX, VIRTIO_RING_F_INDIRECT_DESC,
};
use vm_memory::{GuestMemoryAtomic, GuestMemoryMmap};
use vmm_sys_util::epoll::EventSet;
use vmm_sys_util::eventfd::EventFd;

/// Number of queues we expose. virtio-blk single-queue.
const NUM_QUEUES: usize = 1;
/// Maximum descriptor chain depth per request. virtio-blk descriptor chain
/// is typically 3: outhdr (R), data (R/W), inhdr (W). Indirect chains
/// raise this; 256 is a generous bound.
const MAX_QUEUE_SIZE: u16 = 256;

/// `vhost_user_backend::VhostUserBackend` impl for raftblk.
///
/// Holds the `BlockBackend` and the tokio `Handle` used to drive async
/// dispatch from the sync trait. Memory and event-idx state live behind
/// a `Mutex` because the trait is `&self` (the daemon framework invokes
/// it from multiple threads: memory updates, queue events, exit signal).
pub struct RaftBlkVhostBackend<B: BlockBackend> {
    pub backend: Arc<B>,
    inner: StdMutex<Inner>,
    #[allow(dead_code)]
    runtime: tokio::runtime::Handle,
    exit_event: EventFd,
}

struct Inner {
    mem: Option<GuestMemoryAtomic<GuestMemoryMmap<()>>>,
    event_idx: bool,
}

impl<B: BlockBackend> RaftBlkVhostBackend<B> {
    pub fn new(backend: Arc<B>, runtime: tokio::runtime::Handle, exit_event: EventFd) -> Self {
        Self {
            backend,
            inner: StdMutex::new(Inner {
                mem: None,
                event_idx: false,
            }),
            runtime,
            exit_event,
        }
    }

    /// Whether the EVENT_IDX feature is currently negotiated. Exposed
    /// for the chain-handling implementation to compute the correct
    /// notification policy.
    pub fn event_idx_enabled(&self) -> bool {
        self.inner.lock().unwrap().event_idx
    }
}

impl<B: BlockBackend> VhostUserBackend for RaftBlkVhostBackend<B> {
    type Bitmap = ();
    type Vring = VringRwLock;

    fn num_queues(&self) -> usize {
        NUM_QUEUES
    }
    fn max_queue_size(&self) -> usize {
        MAX_QUEUE_SIZE as usize
    }
    fn features(&self) -> u64 {
        (1u64 << VIRTIO_F_VERSION_1)
            | (1u64 << VIRTIO_BLK_F_BLK_SIZE)
            | (1u64 << VIRTIO_BLK_F_FLUSH)
            | (1u64 << VIRTIO_BLK_F_SEG_MAX)
            | (1u64 << VIRTIO_RING_F_EVENT_IDX)
            | (1u64 << VIRTIO_RING_F_INDIRECT_DESC)
    }
    fn protocol_features(&self) -> VhostUserProtocolFeatures {
        VhostUserProtocolFeatures::CONFIG | VhostUserProtocolFeatures::MQ
    }
    fn set_event_idx(&self, enabled: bool) {
        self.inner.lock().unwrap().event_idx = enabled;
    }
    fn update_memory(&self, mem: GuestMemoryAtomic<GuestMemoryMmap<()>>) -> io::Result<()> {
        self.inner.lock().unwrap().mem = Some(mem);
        Ok(())
    }

    /// Wire-format virtio_blk_config. We assemble the bytes manually
    /// (LE, padded) rather than relying on `ByteValued::as_slice` because
    /// `virtio_blk_config` is foreign and we can't add the impl to it
    /// here. The two relevant fields are `capacity` (8 bytes, LE,
    /// 512-byte sectors) and `blk_size` (4 bytes, LE, after a 32-byte
    /// gap of size_max + seg_max + geometry, before
    /// physical_block_exp).
    ///
    /// This produces a 60-byte buffer that matches what the bindings
    /// struct serializes to; the trailing fields (alignment_offset,
    /// min_io_size, opt_io_size, writeback, ...) are zero, which is
    /// fine for a non-zoned, non-discard, non-WCE device.
    fn get_config(&self, offset: u32, size: u32) -> Vec<u8> {
        let mut bytes = [0u8; std::mem::size_of::<virtio_blk_config>()];
        let capacity_sectors = self.backend.capacity_bytes() / 512;
        bytes[0..8].copy_from_slice(&capacity_sectors.to_le_bytes());
        // size_max (4 bytes) at offset 8 — leave 0 (no per-segment cap).
        // seg_max (4 bytes) at offset 12.
        bytes[12..16].copy_from_slice(&128u32.to_le_bytes());
        // geometry (4 bytes) at 16-20 — zero is fine for non-CHS.
        // blk_size (4 bytes) at offset 20.
        bytes[20..24].copy_from_slice(&(self.backend.block_size() as u32).to_le_bytes());
        let start = (offset as usize).min(bytes.len());
        let end = ((offset + size) as usize).min(bytes.len());
        bytes[start..end].to_vec()
    }

    /// Stub: this is the one piece operator validation has to land
    /// alongside a real vhost-user-master. The `BlockBackend::dispatch`
    /// data plane is fully tested; this trait method is the wire-protocol
    /// glue. The runbook references the exact call sites; until it lands
    /// the daemon will simply not service guest I/O (the guest will time
    /// out the request, the daemon logs a warning).
    fn handle_event(
        &self,
        device_event: u16,
        _evset: EventSet,
        _vrings: &[Self::Vring],
        _thread_id: usize,
    ) -> io::Result<()> {
        log::warn!(
            "raftblk-vhost: handle_event(device_event={device_event}) called, but the \
             vhost-user descriptor-chain handler is not yet wired. See \
             docs/runbooks/raft-block-microvm-smoke.md."
        );
        Ok(())
    }

    fn exit_event(
        &self,
        _thread_index: usize,
    ) -> Option<(
        vmm_sys_util::event::EventConsumer,
        vmm_sys_util::event::EventNotifier,
    )> {
        // Both halves are just clones of our internal exit eventfd. The
        // EventConsumer/EventNotifier types in vmm-sys-util 0.15 take
        // ownership of a raw fd; we hand each one its own dup.
        use std::os::fd::{FromRawFd, IntoRawFd};
        let consumer_fd = self.exit_event.try_clone().ok()?.into_raw_fd();
        let notifier_fd = self.exit_event.try_clone().ok()?.into_raw_fd();
        // SAFETY: we own each fd via try_clone; FromRawFd takes
        // ownership and the events module's Drop closes them.
        let consumer = unsafe { vmm_sys_util::event::EventConsumer::from_raw_fd(consumer_fd) };
        let notifier = unsafe { vmm_sys_util::event::EventNotifier::from_raw_fd(notifier_fd) };
        Some((consumer, notifier))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::InMemoryBlockBackend;
    use uuid::Uuid;

    fn make_backend() -> RaftBlkVhostBackend<InMemoryBlockBackend> {
        let runtime =
            tokio::runtime::Handle::try_current().expect("tests must run inside a tokio runtime");
        let backend = Arc::new(InMemoryBlockBackend::new(
            Uuid::new_v4(),
            4096,
            16 * 1024 * 1024,
        ));
        let exit_event = EventFd::new(0).unwrap();
        RaftBlkVhostBackend::new(backend, runtime, exit_event)
    }

    /// virtio_blk_config wire bytes contain capacity (sectors) at 0..8
    /// and blk_size at 20..24, both little-endian.
    #[tokio::test]
    async fn config_layout_packs_capacity_and_blk_size_at_correct_offsets() {
        let dev = make_backend();
        let bytes = dev.get_config(0, std::mem::size_of::<virtio_blk_config>() as u32);
        // 16 MiB / 512 = 32768 sectors
        let capacity_sectors = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        assert_eq!(capacity_sectors, 32_768);
        let blk_size = u32::from_le_bytes(bytes[20..24].try_into().unwrap());
        assert_eq!(blk_size, 4096);
        let seg_max = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        assert_eq!(seg_max, 128);
    }

    #[tokio::test]
    async fn config_offset_and_size_are_clamped_to_struct_length() {
        let dev = make_backend();
        let total = std::mem::size_of::<virtio_blk_config>() as u32;
        // Reading past the end yields a truncated slice rather than a
        // panic; matches what vhost-user clients expect when probing an
        // older device that only implements a subset of the config space.
        let bytes = dev.get_config(total - 4, 16);
        assert_eq!(bytes.len(), 4);
    }

    #[tokio::test]
    async fn features_advertise_blk_size_flush_seg_max_event_idx() {
        let dev = make_backend();
        let f = dev.features();
        assert!(f & (1 << VIRTIO_F_VERSION_1) != 0);
        assert!(f & (1 << VIRTIO_BLK_F_BLK_SIZE) != 0);
        assert!(f & (1 << VIRTIO_BLK_F_FLUSH) != 0);
        assert!(f & (1 << VIRTIO_BLK_F_SEG_MAX) != 0);
        assert!(f & (1 << VIRTIO_RING_F_EVENT_IDX) != 0);
        // Features we deliberately don't claim:
        assert!(
            f & (1 << VIRTIO_BLK_F_RO) == 0,
            "must not advertise read-only"
        );
        assert!(f & (1 << VIRTIO_BLK_F_MQ) == 0, "single queue only");
    }

    #[tokio::test]
    async fn set_event_idx_round_trips() {
        let dev = make_backend();
        assert!(!dev.event_idx_enabled());
        dev.set_event_idx(true);
        assert!(dev.event_idx_enabled());
        dev.set_event_idx(false);
        assert!(!dev.event_idx_enabled());
    }
}
