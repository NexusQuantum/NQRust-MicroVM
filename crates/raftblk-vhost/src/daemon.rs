//! vhost-user-blk daemon backend wrapping a `BlockBackend`.
//!
//! `RaftBlkVhostBackend` implements `vhost_user_backend::VhostUserBackend`
//! and is wired through `VhostUserDaemon::new(...).serve(socket)` in the
//! binary at `apps/raftblk-vhost`. Each guest virtio-blk request flows:
//!
//!   guest VM → vhost-user socket → daemon's handle_event →
//!   process_queue → handle_chain → BlockBackend::dispatch →
//!   (Raft client_write or local read) → response back through the chain
//!
//! What this module DOES
//! ---------------------
//! - Reports virtio features:
//!   `VIRTIO_F_VERSION_1 | VIRTIO_BLK_F_BLK_SIZE | VIRTIO_BLK_F_FLUSH |
//!    VIRTIO_BLK_F_SEG_MAX | VIRTIO_RING_F_EVENT_IDX |
//!    VIRTIO_RING_F_INDIRECT_DESC`.
//! - Reports vhost-user protocol features: `CONFIG | MQ`.
//! - Builds `virtio_blk_config` (capacity in 512-byte sectors, blk_size,
//!   seg_max=128) via manual LE packing (the bindings struct is foreign
//!   so we can't impl `ByteValued` on it directly).
//! - Drains the queue per kick (`process_queue`) with
//!   disable/enable_notification book-ending so chains arriving during
//!   handling are not missed.
//! - Walks each descriptor chain (`handle_chain`):
//!   - splits readable vs writable halves via `DescriptorChain::reader`/
//!     `writer` from `virtio_queue::descriptor_utils`,
//!   - reads `virtio_blk_outhdr` (16 bytes), extracts type + sector,
//!   - dispatches READ/WRITE/FLUSH/GET_ID through `BlockBackend::dispatch`
//!     (returns `VIRTIO_BLK_S_UNSUPP` for unknown request types),
//!   - copies response data into the writable half (READ/GET_ID only),
//!   - writes the status byte at the end.
//!
//! Tests
//! -----
//! - `handle_chain_executes_virtio_blk_write_through_backend`: builds a
//!   real `MockSplitQueue` with a 3-descriptor chain (outhdr+data+inhdr),
//!   asserts the InMemoryBlockBackend recorded the write at the correct
//!   offset and the status byte is `S_OK`.
//! - `handle_chain_executes_virtio_blk_read_through_backend`: same shape
//!   for IN, asserts the data buffer in guest memory contains the bytes
//!   the backend stored.
//! - `handle_chain_returns_unsupp_for_unknown_request_type`: status byte
//!   is `S_UNSUPP` for unknown request types.
//! - `handle_chain_processes_flush`: status byte is `S_OK`; flush is a
//!   no-op because Raft `client_write` returns synchronously on commit.
//!
//! What still requires operator hardware
//! -------------------------------------
//! Booting a real Firecracker guest with `vhost_user_blk_socket = ...`
//! pointing at this daemon — the runbook at
//! `docs/runbooks/raft-block-microvm-smoke.md` covers prereqs (kernel
//! modules, hugepages, SPDK, 3-host setup). The data plane in this file
//! is exercised end-to-end at the chain level by the unit tests above.

use crate::backend::{BlockBackend, BlockBackendError};
use crate::request::{
    parse_request, BlockRequestKind, BlockResponse, RequestError, VirtioBlkStatus,
};
use std::io;
use std::io::Read;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use vhost::vhost_user::message::{VhostUserProtocolFeatures, VhostUserVirtioFeatures};
use vhost_user_backend::{VhostUserBackend, VringRwLock, VringT};
use virtio_bindings::bindings::virtio_blk::*;
use virtio_bindings::bindings::virtio_config::VIRTIO_F_VERSION_1;
use virtio_bindings::bindings::virtio_ring::{
    VIRTIO_RING_F_EVENT_IDX, VIRTIO_RING_F_INDIRECT_DESC,
};
use virtio_queue::QueueOwnedT;
use vm_memory::{ByteValued, GuestMemoryAtomic, GuestMemoryMmap};
use vmm_sys_util::epoll::EventSet;
use vmm_sys_util::eventfd::EventFd;

/// Newtype wrapper for `virtio_blk_outhdr` so we can `unsafe impl
/// ByteValued`. The bindings struct is `#[repr(C)]` with three integer
/// fields and no padding; every bit pattern is a valid Rust value.
#[repr(transparent)]
#[derive(Debug, Default, Copy, Clone)]
struct VirtioBlkOutHdr(virtio_blk_outhdr);

// SAFETY: virtio_blk_outhdr is `#[repr(C)]`, contains only u32/u64 fields
// (le32/le64 in the bindings, but those are u32/u64 newtypes), has no
// padding, and every bit pattern is a valid value.
unsafe impl ByteValued for VirtioBlkOutHdr {}

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
        // VHOST_USER_F_PROTOCOL_FEATURES (bit 30) MUST be set for the
        // daemon to negotiate protocol-level features (REPLY_ACK,
        // VRING_ENABLE flow, etc.). Without it the master can connect
        // but cannot activate vrings; vhost-user-backend's set_vring_enable
        // hook returns "inactive feature: 1073741824" and the device
        // never comes online.
        VhostUserVirtioFeatures::PROTOCOL_FEATURES.bits()
            | (1u64 << VIRTIO_F_VERSION_1)
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

    fn handle_event(
        &self,
        device_event: u16,
        _evset: EventSet,
        vrings: &[Self::Vring],
        _thread_id: usize,
    ) -> io::Result<()> {
        if device_event != 0 {
            return Err(io::Error::other(format!(
                "raftblk-vhost: unexpected device event {device_event}"
            )));
        }
        let vring = &vrings[0];
        let mem_atomic = self
            .inner
            .lock()
            .unwrap()
            .mem
            .clone()
            .ok_or_else(|| io::Error::other("raftblk-vhost: memory not yet set"))?;
        process_queue(self, vring, &mem_atomic)
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

/// Drain the vring's pending descriptor chains. Loops with
/// disable_notification / enable_notification so any chain that arrives
/// between iterations is not missed (standard EVENT_IDX-safe pattern).
fn process_queue<B: BlockBackend>(
    backend: &RaftBlkVhostBackend<B>,
    vring: &VringRwLock,
    mem_atomic: &GuestMemoryAtomic<GuestMemoryMmap<()>>,
) -> io::Result<()> {
    use vm_memory::GuestAddressSpace;
    let mem = mem_atomic.memory();
    let mut needs_signal = false;
    loop {
        vring
            .disable_notification()
            .map_err(|e| io::Error::other(format!("disable_notification: {e:?}")))?;

        // Collect the chains under a short-lived lock so we don't hold
        // it across the async backend dispatch.
        let mut chains_to_process = Vec::new();
        {
            let mut state = vring.get_mut();
            let queue = state.get_queue_mut();
            let chains = queue
                .iter(mem.clone())
                .map_err(|e| io::Error::other(format!("queue iter: {e:?}")))?;
            for chain in chains {
                chains_to_process.push(chain);
            }
        }
        if chains_to_process.is_empty() {
            if !vring
                .enable_notification()
                .map_err(|e| io::Error::other(format!("enable_notification: {e:?}")))?
            {
                break;
            }
            continue;
        }

        for chain in chains_to_process {
            let head_idx = chain.head_index();
            // The daemon's worker thread is not a tokio runtime thread,
            // so block_on here is correct (panics only when invoked from
            // within an active tokio worker). Tests use `.await`
            // directly via the async helper.
            let used_len = match backend.runtime.block_on(handle_chain(backend, chain)) {
                Ok(len) => len,
                Err(err) => {
                    log::error!("raftblk-vhost: chain handling failed: {err}");
                    0
                }
            };
            vring
                .add_used(head_idx, used_len)
                .map_err(|e| io::Error::other(format!("add_used: {e:?}")))?;
            needs_signal = true;
        }
    }

    if needs_signal {
        vring
            .signal_used_queue()
            .map_err(|e| io::Error::other(format!("signal_used_queue: {e:?}")))?;
    }
    Ok(())
}

/// Process one virtio-blk descriptor chain. Returns the number of bytes
/// the device wrote into the chain (used for the used-ring length).
///
/// Layout (per virtio 1.1 §5.2):
///   - readable: virtio_blk_outhdr (16 bytes) + optional data buffer (for OUT)
///   - writable: optional data buffer (for IN/GET_ID) + virtio_blk_inhdr (1 byte)
///
/// Async because backend.dispatch is async (Raft commit). The daemon's
/// sync handle_event uses `runtime.block_on` on a non-tokio worker
/// thread; tests `.await` directly.
async fn handle_chain<B: BlockBackend, M>(
    backend: &RaftBlkVhostBackend<B>,
    chain: virtio_queue::DescriptorChain<M>,
) -> Result<u32, ChainError>
where
    M: std::ops::Deref + Clone,
    M::Target: vm_memory::GuestMemory + Sized,
{
    // Build reader + writer over copies of the chain handle. Each split
    // consumes its chain via the readable() / writable() iterator, so we
    // need two copies. The chain is Clone-able and cheap (just indices).
    let chain_for_reader = chain.clone();
    let chain_for_writer = chain;
    let mem_ref = chain_for_reader.memory() as *const _;
    // SAFETY: we only use mem_ref to satisfy reader/writer's lifetime
    // requirement; both end consumers (reader, writer) outlive only this
    // function, and the underlying GuestMemory is held alive by the
    // chain's `mem: M` field which lives through the whole function.
    let mem = unsafe { &*mem_ref };
    let mut reader = chain_for_reader
        .reader(mem)
        .map_err(|e| ChainError::ChainSplit(format!("reader: {e:?}")))?;
    let mut writer = chain_for_writer
        .writer(mem)
        .map_err(|e| ChainError::ChainSplit(format!("writer: {e:?}")))?;

    if reader.available_bytes() < std::mem::size_of::<virtio_blk_outhdr>() {
        return Err(ChainError::ShortHeader(reader.available_bytes()));
    }
    if writer.available_bytes() < 1 {
        return Err(ChainError::NoStatusByte);
    }

    let outhdr: VirtioBlkOutHdr = reader
        .read_obj()
        .map_err(|e| ChainError::Memory(format!("read outhdr: {e}")))?;
    let req_type = outhdr.0.type_;
    let sector = outhdr.0.sector;

    // Read any remaining readable bytes (the data buffer for OUT).
    let readable_data_len = reader.available_bytes();
    let mut readable_data = vec![0u8; readable_data_len];
    if readable_data_len > 0 {
        reader
            .read_exact(&mut readable_data)
            .map_err(|e| ChainError::Memory(format!("read data: {e}")))?;
    }

    // Available writable bytes minus the trailing status byte.
    let writable_total = writer.available_bytes();
    let writable_data_len = writable_total.saturating_sub(1);

    let block_size = backend.backend.block_size();
    let req = match parse_request(
        req_type,
        sector,
        block_size,
        writable_data_len as u32,
        &readable_data,
    ) {
        Ok(r) => r,
        Err(RequestError::UnsupportedType(_)) => {
            // Skip past data buffer (writer cursor stays at start of
            // writable region; we still need to land the status byte at
            // the end). We just write zeros for the data part and the
            // status byte.
            if writable_data_len > 0 {
                writer
                    .write_all(&vec![0u8; writable_data_len])
                    .map_err(|e| ChainError::Memory(format!("zero pad: {e}")))?;
            }
            writer
                .write_all(&[VirtioBlkStatus::Unsupp as u8])
                .map_err(|e| ChainError::Memory(format!("status: {e}")))?;
            return Ok(writer.bytes_written() as u32);
        }
        Err(_) => {
            if writable_data_len > 0 {
                writer
                    .write_all(&vec![0u8; writable_data_len])
                    .map_err(|e| ChainError::Memory(format!("zero pad: {e}")))?;
            }
            writer
                .write_all(&[VirtioBlkStatus::IoErr as u8])
                .map_err(|e| ChainError::Memory(format!("status: {e}")))?;
            return Ok(writer.bytes_written() as u32);
        }
    };

    // Dispatch through the async backend.
    let dispatch = backend.backend.dispatch(req.clone()).await;
    let response: BlockResponse = match dispatch {
        Ok(r) => r,
        Err(BlockBackendError::Transport(e)) => {
            log::error!("raftblk-vhost: backend transport: {e}");
            if writable_data_len > 0 {
                writer
                    .write_all(&vec![0u8; writable_data_len])
                    .map_err(|e| ChainError::Memory(format!("zero pad: {e}")))?;
            }
            writer
                .write_all(&[VirtioBlkStatus::IoErr as u8])
                .map_err(|e| ChainError::Memory(format!("status: {e}")))?;
            return Ok(writer.bytes_written() as u32);
        }
        Err(other) => {
            log::error!("raftblk-vhost: backend rejected: {other}");
            if writable_data_len > 0 {
                writer
                    .write_all(&vec![0u8; writable_data_len])
                    .map_err(|e| ChainError::Memory(format!("zero pad: {e}")))?;
            }
            writer
                .write_all(&[VirtioBlkStatus::IoErr as u8])
                .map_err(|e| ChainError::Memory(format!("status: {e}")))?;
            return Ok(writer.bytes_written() as u32);
        }
    };

    // Write response data into the writable data half (for IN / GET_ID).
    match req.kind {
        BlockRequestKind::Read { .. } | BlockRequestKind::GetId => {
            let data = response.data.as_slice();
            // Pad/truncate to writable_data_len so the write_all consumes
            // exactly the data half before the status byte.
            if data.len() == writable_data_len {
                writer
                    .write_all(data)
                    .map_err(|e| ChainError::Memory(format!("write data: {e}")))?;
            } else if data.len() < writable_data_len {
                writer
                    .write_all(data)
                    .map_err(|e| ChainError::Memory(format!("write data: {e}")))?;
                writer
                    .write_all(&vec![0u8; writable_data_len - data.len()])
                    .map_err(|e| ChainError::Memory(format!("pad data: {e}")))?;
            } else {
                // Backend produced more data than the chain can hold.
                // Truncate to fit and report IoErr to the guest so the
                // partial data isn't mistaken for success.
                writer
                    .write_all(&data[..writable_data_len])
                    .map_err(|e| ChainError::Memory(format!("trunc data: {e}")))?;
                writer
                    .write_all(&[VirtioBlkStatus::IoErr as u8])
                    .map_err(|e| ChainError::Memory(format!("status: {e}")))?;
                return Ok(writer.bytes_written() as u32);
            }
        }
        BlockRequestKind::Write { .. } | BlockRequestKind::Flush => {
            // Writer cursor is already at the trailing status byte (no
            // writable data half for write/flush requests).
            if writable_data_len > 0 {
                // Defensive: if the guest exposed a writable buffer for
                // a write/flush, just zero it.
                writer
                    .write_all(&vec![0u8; writable_data_len])
                    .map_err(|e| ChainError::Memory(format!("zero pad: {e}")))?;
            }
        }
    }

    writer
        .write_all(&[response.status as u8])
        .map_err(|e| ChainError::Memory(format!("write status: {e}")))?;
    Ok(writer.bytes_written() as u32)
}

#[derive(Debug, thiserror::Error)]
pub enum ChainError {
    #[error("descriptor chain split failed: {0}")]
    ChainSplit(String),
    #[error("readable region too short for virtio_blk_outhdr ({0} bytes)")]
    ShortHeader(usize),
    #[error("writable region missing trailing status byte")]
    NoStatusByte,
    #[error("guest memory error: {0}")]
    Memory(String),
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

    // ------- Real virtqueue / handle_chain tests -------
    //
    // These build descriptor chains in a real GuestMemoryMmap using
    // virtio-queue's MockSplitQueue and drive them through handle_chain.
    // No actual vhost-user master is needed; this proves the descriptor
    // walk + Reader/Writer split + virtio-blk header decode + backend
    // dispatch + status byte writeback all line up.

    use virtio_bindings::bindings::virtio_ring::{VRING_DESC_F_NEXT, VRING_DESC_F_WRITE};
    use virtio_queue::desc::split::Descriptor as SplitDescriptor;
    use virtio_queue::desc::RawDescriptor;
    use virtio_queue::mock::MockSplitQueue;
    use vm_memory::{Bytes, GuestAddress, GuestMemoryMmap};

    /// Build a `GuestMemoryMmap` covering offsets 0..0x100000 and a
    /// helper that lets us write/read at arbitrary guest addresses.
    fn make_guest_memory() -> GuestMemoryMmap<()> {
        GuestMemoryMmap::<()>::from_ranges(&[(GuestAddress(0x0), 0x100000)]).unwrap()
    }

    /// Build a virtio-blk OUT (write) chain: outhdr → data → inhdr.
    /// Returns the chain plus the GuestMemoryMmap so the caller can
    /// inspect the inhdr byte after the handler runs.
    #[tokio::test]
    async fn handle_chain_executes_virtio_blk_write_through_backend() {
        let mem = make_guest_memory();
        let outhdr_addr = GuestAddress(0x10000);
        let data_addr = GuestAddress(0x11000);
        let inhdr_addr = GuestAddress(0x12000);

        // Write the outhdr in guest memory: type=OUT, sector=0.
        let outhdr = virtio_blk_outhdr {
            type_: VIRTIO_BLK_T_OUT,
            ioprio: 0,
            sector: 0,
        };
        mem.write_obj(VirtioBlkOutHdr(outhdr), outhdr_addr).unwrap();
        // Write the payload: 4096 bytes of 0xab. Block size is 4096 so
        // this is one full block at offset 0.
        mem.write_slice(&vec![0xab; 4096], data_addr).unwrap();

        let queue = MockSplitQueue::new(&mem, 16);
        let descs = vec![
            // outhdr: readable, len 16 (size_of virtio_blk_outhdr)
            RawDescriptor::from(SplitDescriptor::new(
                outhdr_addr.0,
                16,
                VRING_DESC_F_NEXT as u16,
                1,
            )),
            // data: readable (write-from-device-to-storage; the
            // direction the OUT type implies is that the device READS
            // from this buffer, so no F_WRITE here)
            RawDescriptor::from(SplitDescriptor::new(
                data_addr.0,
                4096,
                VRING_DESC_F_NEXT as u16,
                2,
            )),
            // inhdr: writable, 1 byte for status
            RawDescriptor::from(SplitDescriptor::new(
                inhdr_addr.0,
                1,
                VRING_DESC_F_WRITE as u16,
                0,
            )),
        ];
        let chain = queue.build_desc_chain(&descs).unwrap();

        let dev = make_backend();
        let bytes_written = handle_chain(&dev, chain)
            .await
            .expect("chain handles cleanly");

        // For an OUT request, only the status byte is written by the
        // device, so bytes_written == 1.
        assert_eq!(bytes_written, 1, "write request used-len");

        // The status byte should be VIRTIO_BLK_S_OK = 0.
        let status: u8 = mem.read_obj(inhdr_addr).unwrap();
        assert_eq!(status, VirtioBlkStatus::Ok as u8);

        // The InMemoryBlockBackend recorded the write.
        let log = dev.backend.write_log();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].0, 0, "guest wrote at sector 0 -> byte offset 0");
        assert_eq!(log[0].1.len(), 4096);
        assert_eq!(log[0].1[0], 0xab);
    }

    /// virtio-blk IN (read) chain: outhdr (readable) → data (writable)
    /// → inhdr (writable). The device fills the data buffer from the
    /// backend then writes the status byte.
    #[tokio::test]
    async fn handle_chain_executes_virtio_blk_read_through_backend() {
        let mem = make_guest_memory();
        let outhdr_addr = GuestAddress(0x10000);
        let data_addr = GuestAddress(0x11000);
        let inhdr_addr = GuestAddress(0x12000);

        let outhdr = virtio_blk_outhdr {
            type_: VIRTIO_BLK_T_IN,
            ioprio: 0,
            sector: 8, // sector 8 * 512 = byte offset 4096
        };
        mem.write_obj(VirtioBlkOutHdr(outhdr), outhdr_addr).unwrap();

        let queue = MockSplitQueue::new(&mem, 16);
        let descs = vec![
            RawDescriptor::from(SplitDescriptor::new(
                outhdr_addr.0,
                16,
                VRING_DESC_F_NEXT as u16,
                1,
            )),
            RawDescriptor::from(SplitDescriptor::new(
                data_addr.0,
                4096,
                (VRING_DESC_F_WRITE | VRING_DESC_F_NEXT) as u16,
                2,
            )),
            RawDescriptor::from(SplitDescriptor::new(
                inhdr_addr.0,
                1,
                VRING_DESC_F_WRITE as u16,
                0,
            )),
        ];
        let chain = queue.build_desc_chain(&descs).unwrap();

        // Pre-populate the in-memory backend so the read returns
        // recognizable bytes.
        let dev = make_backend();
        // Issue a write through the backend to populate offset 4096 with
        // 0x55 (matches sector 8 = byte 4096 from above).
        dev.backend
            .dispatch(crate::request::BlockRequest {
                sector: 8,
                kind: BlockRequestKind::Write {
                    offset: 4096,
                    data: vec![0x55; 4096],
                },
            })
            .await
            .unwrap();

        let bytes_written = handle_chain(&dev, chain)
            .await
            .expect("read chain handles cleanly");
        assert_eq!(bytes_written, 4096 + 1, "read used-len = data + status");

        // Status OK.
        let status: u8 = mem.read_obj(inhdr_addr).unwrap();
        assert_eq!(status, VirtioBlkStatus::Ok as u8);

        // The data buffer in guest memory should contain 0x55s.
        let mut buf = vec![0u8; 4096];
        mem.read_slice(&mut buf, data_addr).unwrap();
        assert!(
            buf.iter().all(|&b| b == 0x55),
            "guest read returned the bytes the backend stored"
        );
    }

    /// Unsupported request types (e.g. discard) get VIRTIO_BLK_S_UNSUPP
    /// without crashing the daemon.
    #[tokio::test]
    async fn handle_chain_returns_unsupp_for_unknown_request_type() {
        let mem = make_guest_memory();
        let outhdr_addr = GuestAddress(0x10000);
        let inhdr_addr = GuestAddress(0x11000);

        let outhdr = virtio_blk_outhdr {
            type_: 999, // not a real virtio_blk type
            ioprio: 0,
            sector: 0,
        };
        mem.write_obj(VirtioBlkOutHdr(outhdr), outhdr_addr).unwrap();

        let queue = MockSplitQueue::new(&mem, 16);
        let descs = vec![
            RawDescriptor::from(SplitDescriptor::new(
                outhdr_addr.0,
                16,
                VRING_DESC_F_NEXT as u16,
                1,
            )),
            RawDescriptor::from(SplitDescriptor::new(
                inhdr_addr.0,
                1,
                VRING_DESC_F_WRITE as u16,
                0,
            )),
        ];
        let chain = queue.build_desc_chain(&descs).unwrap();

        let dev = make_backend();
        let bytes_written = handle_chain(&dev, chain)
            .await
            .expect("unknown type doesn't crash");
        assert_eq!(bytes_written, 1);
        let status: u8 = mem.read_obj(inhdr_addr).unwrap();
        assert_eq!(status, VirtioBlkStatus::Unsupp as u8);
    }

    /// FLUSH is a no-op that always returns OK (the underlying Raft
    /// commit is synchronous so prior writes are already durable).
    #[tokio::test]
    async fn handle_chain_processes_flush() {
        let mem = make_guest_memory();
        let outhdr_addr = GuestAddress(0x10000);
        let inhdr_addr = GuestAddress(0x11000);

        let outhdr = virtio_blk_outhdr {
            type_: VIRTIO_BLK_T_FLUSH,
            ioprio: 0,
            sector: 0,
        };
        mem.write_obj(VirtioBlkOutHdr(outhdr), outhdr_addr).unwrap();

        let queue = MockSplitQueue::new(&mem, 16);
        let descs = vec![
            RawDescriptor::from(SplitDescriptor::new(
                outhdr_addr.0,
                16,
                VRING_DESC_F_NEXT as u16,
                1,
            )),
            RawDescriptor::from(SplitDescriptor::new(
                inhdr_addr.0,
                1,
                VRING_DESC_F_WRITE as u16,
                0,
            )),
        ];
        let chain = queue.build_desc_chain(&descs).unwrap();

        let dev = make_backend();
        let bytes_written = handle_chain(&dev, chain)
            .await
            .expect("flush handles cleanly");
        assert_eq!(bytes_written, 1);
        let status: u8 = mem.read_obj(inhdr_addr).unwrap();
        assert_eq!(status, VirtioBlkStatus::Ok as u8);
    }
}
