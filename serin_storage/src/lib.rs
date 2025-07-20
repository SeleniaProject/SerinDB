//! SerinDB storage layer primitives.
#![deny(missing_docs)]

use crc32c::crc32c;
use serde::{Deserialize, Serialize};

pub mod buffer;

/// Default page size (bytes).
pub const PAGE_SIZE: usize = 16 * 1024; // 16 KiB

/// Page header as stored on disk.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub struct PageHeader {
    /// Type of the page (e.g., table leaf, index internal, etc.).
    pub page_type: u16,
    /// CRC32C checksum of the entire page (set to 0 during calculation).
    pub checksum: u16,
    /// Log sequence number for WAL.
    pub lsn: u32,
    /// Number of tuple slots.
    pub slot_count: u16,
    /// Offset to the start of free space (grows backward).
    pub free_space_offset: u16,
}

impl Default for PageHeader {
    fn default() -> Self {
        Self {
            page_type: 0,
            checksum: 0,
            lsn: 0,
            slot_count: 0,
            free_space_offset: PAGE_SIZE as u16,
        }
    }
}

/// Tuple slot entry in the slot directory.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TupleSlot {
    /// Offset from page start to tuple data.
    pub offset: u16,
    /// Tuple length in bytes.
    pub length: u16,
}

/// Compute CRC32C checksum for a page buffer (header `checksum` field must be zeroed).
pub fn compute_checksum(page: &[u8]) -> u16 {
    let sum = crc32c(page);
    // Fold 32-bit CRC into 16-bit value (as PostgreSQL does).
    ((sum >> 16) as u16) ^ (sum as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_roundtrip() {
        let mut page = vec![0u8; PAGE_SIZE];
        let header = PageHeader::default();
        // Serialize header at page start.
        let hdr_bytes = bincode::serialize(&header).unwrap();
        page[..hdr_bytes.len()].copy_from_slice(&hdr_bytes);

        // Calculate checksum with zeroed field.
        let csum = compute_checksum(&page);

        // Write checksum back into header in page buffer.
        page[2..4].copy_from_slice(&csum.to_le_bytes());

        // Verify checksum matches after embedding.
        assert_eq!(csum, compute_checksum(&page));
    }
} 