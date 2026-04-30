//! Translation between virtio-blk descriptor-chain shaped requests and
//! `BlockBackend` operations.
//!
//! virtio-blk request layout (per virtio 1.1 §5.2):
//!
//! ```text
//! struct virtio_blk_outhdr {
//!     le32 type;       // VIRTIO_BLK_T_IN/OUT/FLUSH/...
//!     le32 reserved;
//!     le64 sector;     // 512-byte logical sector
//! }
//! // ... data buffer (read or written) ...
//! struct virtio_blk_inhdr {
//!     u8 status;       // VIRTIO_BLK_S_OK / IOERR / UNSUPP
//! }
//! ```
//!
//! The daemon parses descriptor chains into `BlockRequest`, dispatches to
//! the backend, and produces a `BlockResponse` whose `status` byte is what
//! the inhdr descriptor must be filled with before notifying the guest.
//!
//! All lengths and offsets are converted to bytes here, in terms of the
//! Raft group's `block_size`. The 512-byte virtio sector is multiplied by
//! the on-the-wire sector count; alignment to `block_size` is enforced
//! before any backend call.

use crate::VIRTIO_BLK_SECTOR_SIZE;
use thiserror::Error;

/// virtio_blk_req.type values (subset; we don't claim discard/zeroes/secure
/// erase support yet).
pub const VIRTIO_BLK_T_IN: u32 = 0;
pub const VIRTIO_BLK_T_OUT: u32 = 1;
pub const VIRTIO_BLK_T_FLUSH: u32 = 4;
pub const VIRTIO_BLK_T_GET_ID: u32 = 8;

/// virtio_blk_inhdr.status values.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtioBlkStatus {
    Ok = 0,
    IoErr = 1,
    Unsupp = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockRequestKind {
    /// Read `len` bytes starting at `offset`. Must be `block_size`-aligned.
    Read { offset: u64, len: u32 },
    /// Write `data` at `offset`. Must be `block_size`-aligned.
    Write { offset: u64, data: Vec<u8> },
    /// Persist any in-flight writes. For Raft-backed storage the leader's
    /// `client_write` doesn't return until the entry is committed and applied,
    /// so flush is a no-op and always succeeds.
    Flush,
    /// virtio-blk identification string (20 bytes, padded). Used by guest
    /// kernels for `/sys/block/<dev>/serial`. We return a deterministic id
    /// derived from the group_id so guest tooling can correlate disks to
    /// Raft groups.
    GetId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockRequest {
    /// 512-byte sector from the virtio header. Some kinds (Flush, GetId)
    /// ignore this; for Read/Write it is the source of `offset`.
    pub sector: u64,
    pub kind: BlockRequestKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockResponse {
    pub status: VirtioBlkStatus,
    /// For Read: the bytes returned to the guest data buffer.
    /// For GetId: the 20-byte serial identifier.
    /// For Write/Flush: empty.
    pub data: Vec<u8>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RequestError {
    #[error("unsupported virtio_blk_req type {0}")]
    UnsupportedType(u32),
    #[error("offset {offset} not aligned to block_size {block_size}")]
    UnalignedOffset { offset: u64, block_size: u64 },
    #[error("length {len} not aligned to block_size {block_size}")]
    UnalignedLength { len: u32, block_size: u64 },
    #[error("read length {len} exceeds maximum {max}")]
    ReadTooLarge { len: u32, max: u32 },
    #[error("write length {len} does not match buffer length {buf_len}")]
    WriteLengthMismatch { len: u32, buf_len: usize },
}

/// Build a `BlockRequest` from the virtio header fields plus the data
/// buffer for writes. Performs alignment checks against `block_size` and
/// rejects unsupported request types up-front so the daemon doesn't have
/// to round-trip to the backend just to learn it doesn't support discard.
///
/// `data` is the writable portion of the descriptor chain (for VIRTIO_BLK_T_OUT)
/// or empty (for IN/FLUSH/GET_ID where the data buffer is allocated by the
/// device for filling).
pub fn parse_request(
    req_type: u32,
    sector: u64,
    block_size: u64,
    read_len: u32,
    data: &[u8],
) -> Result<BlockRequest, RequestError> {
    let kind = match req_type {
        VIRTIO_BLK_T_IN => {
            let offset = sector.checked_mul(VIRTIO_BLK_SECTOR_SIZE).ok_or(
                RequestError::UnalignedOffset {
                    offset: sector,
                    block_size,
                },
            )?;
            if !offset.is_multiple_of(block_size) {
                return Err(RequestError::UnalignedOffset { offset, block_size });
            }
            if !(read_len as u64).is_multiple_of(block_size) {
                return Err(RequestError::UnalignedLength {
                    len: read_len,
                    block_size,
                });
            }
            // Sanity bound to refuse pathological reads that would allocate
            // gigabytes on the daemon side. Real virtio-blk requests don't
            // exceed a few MB.
            const MAX_READ: u32 = 16 * 1024 * 1024;
            if read_len > MAX_READ {
                return Err(RequestError::ReadTooLarge {
                    len: read_len,
                    max: MAX_READ,
                });
            }
            BlockRequestKind::Read {
                offset,
                len: read_len,
            }
        }
        VIRTIO_BLK_T_OUT => {
            let offset = sector.checked_mul(VIRTIO_BLK_SECTOR_SIZE).ok_or(
                RequestError::UnalignedOffset {
                    offset: sector,
                    block_size,
                },
            )?;
            if !offset.is_multiple_of(block_size) {
                return Err(RequestError::UnalignedOffset { offset, block_size });
            }
            if !(data.len() as u64).is_multiple_of(block_size) {
                return Err(RequestError::UnalignedLength {
                    len: data.len() as u32,
                    block_size,
                });
            }
            BlockRequestKind::Write {
                offset,
                data: data.to_vec(),
            }
        }
        VIRTIO_BLK_T_FLUSH => BlockRequestKind::Flush,
        VIRTIO_BLK_T_GET_ID => BlockRequestKind::GetId,
        other => return Err(RequestError::UnsupportedType(other)),
    };
    Ok(BlockRequest { sector, kind })
}

/// Format the 20-byte virtio-blk serial id. We pack the group UUID's low 16
/// bytes into the first 16 bytes of the id and pad the remainder. Guests
/// reading `/sys/block/<dev>/serial` see a deterministic identifier they
/// can correlate with the Raft group on the host side.
pub fn format_serial_id(group_id: uuid::Uuid) -> Vec<u8> {
    let mut out = vec![0u8; 20];
    let bytes = group_id.as_bytes();
    out[..16].copy_from_slice(bytes);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn parse_read_request_translates_sector_to_byte_offset() {
        let req = parse_request(VIRTIO_BLK_T_IN, 8, 4096, 4096, &[]).unwrap();
        assert_eq!(req.sector, 8);
        match req.kind {
            BlockRequestKind::Read { offset, len } => {
                // sector 8 * 512 = byte 4096
                assert_eq!(offset, 4096);
                assert_eq!(len, 4096);
            }
            other => panic!("expected Read, got {other:?}"),
        }
    }

    #[test]
    fn parse_write_request_uses_data_buffer_length() {
        let payload = vec![0xa5; 4096];
        let req = parse_request(VIRTIO_BLK_T_OUT, 16, 4096, 0, &payload).unwrap();
        assert_eq!(req.sector, 16);
        match req.kind {
            BlockRequestKind::Write { offset, data } => {
                // sector 16 * 512 = byte 8192
                assert_eq!(offset, 8192);
                assert_eq!(data.len(), 4096);
                assert!(data.iter().all(|&b| b == 0xa5));
            }
            other => panic!("expected Write, got {other:?}"),
        }
    }

    #[test]
    fn parse_rejects_misaligned_read() {
        // sector 1 * 512 = byte 512 — not aligned to block_size 4096
        let err = parse_request(VIRTIO_BLK_T_IN, 1, 4096, 4096, &[]).unwrap_err();
        assert!(matches!(
            err,
            RequestError::UnalignedOffset {
                offset: 512,
                block_size: 4096
            }
        ));
    }

    #[test]
    fn parse_rejects_misaligned_write_length() {
        // 100 bytes is not a multiple of block_size 512
        let err = parse_request(VIRTIO_BLK_T_OUT, 0, 512, 0, &[0u8; 100]).unwrap_err();
        assert!(matches!(
            err,
            RequestError::UnalignedLength {
                len: 100,
                block_size: 512
            }
        ));
    }

    #[test]
    fn parse_rejects_unsupported_type() {
        let err = parse_request(99, 0, 512, 0, &[]).unwrap_err();
        assert_eq!(err, RequestError::UnsupportedType(99));
    }

    #[test]
    fn parse_flush_and_get_id_pass_through_without_alignment_checks() {
        let flush = parse_request(VIRTIO_BLK_T_FLUSH, 0, 4096, 0, &[]).unwrap();
        assert!(matches!(flush.kind, BlockRequestKind::Flush));
        let id = parse_request(VIRTIO_BLK_T_GET_ID, 0, 4096, 0, &[]).unwrap();
        assert!(matches!(id.kind, BlockRequestKind::GetId));
    }

    #[test]
    fn parse_caps_oversized_reads() {
        let err = parse_request(VIRTIO_BLK_T_IN, 0, 512, 100 * 1024 * 1024, &[]).unwrap_err();
        assert!(matches!(err, RequestError::ReadTooLarge { .. }));
    }

    #[test]
    fn format_serial_id_is_20_bytes_and_starts_with_uuid() {
        let id = Uuid::from_u128(0xdead_beef_cafe_f00d_1234_5678_90ab_cdef);
        let serial = format_serial_id(id);
        assert_eq!(serial.len(), 20);
        assert_eq!(&serial[..16], id.as_bytes());
        // Tail is zero-padded.
        assert!(serial[16..].iter().all(|&b| b == 0));
    }
}
