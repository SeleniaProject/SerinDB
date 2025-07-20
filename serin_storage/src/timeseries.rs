//! Time-series storage primitives (Phase 9.3).
//! 
//! This module provides a column-oriented chunk writer, Gorilla-style
//! delta-of-delta compression for timestamps and XOR compression for
//! floating-point values, a simple time-bucket index, and continuous
//! aggregate roll-up infrastructure.
//!
//! The implementation follows the design goals described in the design
//! document and meets the requirements for Phase 9.3 of the task list.

use bitvec::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Logical timestamp type (Unix epoch nanos).
pub type Timestamp = i64;

/// Fixed-width value type for this MVP (f64).
/// In the future this can be extended to arbitrarily typed columns via
/// binary ser/de but we focus on numeric telemetry for now.
pub type Value = f64;

/// Chunk size in rows (fixed for the MVP).
const CHUNK_CAPACITY: usize = 16 * 1024; // 16 K rows per chunk

/// Column-oriented chunk holding one metric series.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnChunk {
    /// Uncompressed timestamps.
    timestamps: Vec<Timestamp>,
    /// Uncompressed values.
    values: Vec<Value>,
}

impl ColumnChunk {
    /// Create a new empty chunk.
    pub fn new() -> Self {
        Self {
            timestamps: Vec::with_capacity(CHUNK_CAPACITY),
            values: Vec::with_capacity(CHUNK_CAPACITY),
        }
    }

    /// Current number of stored rows.
    #[inline]
    pub fn len(&self) -> usize {
        self.timestamps.len()
    }

    /// Whether the chunk is full.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len() >= CHUNK_CAPACITY
    }

    /// Append a single (timestamp, value) pair.
    pub fn append(&mut self, ts: Timestamp, val: Value) {
        self.timestamps.push(ts);
        self.values.push(val);
    }

    /// Compress the current chunk using Gorilla compression.
    pub fn compress(&self) -> CompressedChunk {
        CompressedChunk::from_chunk(self)
    }
}

/// Bit-level buffer used by the Gorilla encoder.
#[derive(Default, Clone)]
struct BitBuffer {
    bits: BitVec<u8, Msb0>,
}

impl BitBuffer {
    #[inline]
    fn push_bit(&mut self, b: bool) {
        self.bits.push(b);
    }

    #[inline]
    fn push_bits(&mut self, value: u64, bits: usize) {
        for i in (0..bits).rev() {
            self.bits.push(((value >> i) & 1) == 1);
        }
    }

    fn into_vec(self) -> Vec<u8> {
        self.bits.into_vec()
    }
}

/// Encoded chunk (timestamps + values).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedChunk {
    /// First timestamp stored raw.
    base_ts: Timestamp,
    /// First value stored raw.
    base_val: Value,
    /// Encoded timestamp diff-stream.
    ts_bits: Vec<u8>,
    /// Encoded value xor-stream.
    val_bits: Vec<u8>,
    /// Number of rows.
    rows: usize,
}

impl CompressedChunk {
    /// Build a compressed chunk from the given column chunk.
    pub fn from_chunk(chunk: &ColumnChunk) -> Self {
        assert!(!chunk.timestamps.is_empty(), "chunk must contain at least one row");

        let rows = chunk.timestamps.len();
        let mut ts_buf = BitBuffer::default();

        // Gorilla timestamp compression
        let mut prev_ts = chunk.timestamps[0];
        let mut prev_delta = 0i64;
        for &ts in &chunk.timestamps[1..] {
            let delta = ts - prev_ts;
            let delta_of_delta = delta - prev_delta;
            prev_ts = ts;
            prev_delta = delta;

            // ZigZag encode delta_of_delta to map signed -> unsigned
            let zz = ((delta_of_delta << 1) ^ (delta_of_delta >> 63)) as u64;
            // Variable bits: write 0 for small, 1 + 12 bits for medium, 2 + 20 bits, else 3 + 64 bits
            if zz == 0 {
                ts_buf.push_bit(false); // control bit 0
            } else {
                ts_buf.push_bit(true); // control bit 1
                let bits = 64 - zz.leading_zeros();
                match bits {
                    0..=12 => {
                        ts_buf.push_bits(0b00, 2);
                        ts_buf.push_bits(zz, 12);
                    }
                    13..=20 => {
                        ts_buf.push_bits(0b01, 2);
                        ts_buf.push_bits(zz, 20);
                    }
                    21..=32 => {
                        ts_buf.push_bits(0b10, 2);
                        ts_buf.push_bits(zz, 32);
                    }
                    _ => {
                        ts_buf.push_bits(0b11, 2);
                        ts_buf.push_bits(zz, 64);
                    }
                }
            }
        }

        // Gorilla value compression
        let mut val_buf = BitBuffer::default();
        let mut prev_val_bits = chunk.values[0].to_bits();
        let mut prev_leading = 64u8;
        let mut prev_trailing = 0u8;

        for &v in &chunk.values[1..] {
            let vb = v.to_bits();
            let xor = prev_val_bits ^ vb;
            if xor == 0 {
                // Write single 0 bit
                val_buf.push_bit(false);
            } else {
                val_buf.push_bit(true);
                let leading = xor.leading_zeros() as u8;
                let trailing = xor.trailing_zeros() as u8;
                if leading >= prev_leading && trailing >= prev_trailing {
                    // Reuse previous leading/trailing block (control 0)
                    val_buf.push_bit(false);
                    let significant_bits = 64 - prev_leading as u32 - prev_trailing as u32;
                    val_buf.push_bits(xor >> prev_trailing, significant_bits as usize);
                } else {
                    // Store new leading/trailing (control 1)
                    val_buf.push_bit(true);
                    val_buf.push_bits(leading as u64, 6); // 6 bits for leading zeros
                    let significant_bits = 64 - leading as u32 - trailing as u32;
                    val_buf.push_bits((significant_bits - 1) as u64, 6); // store length-1 (6 bits)
                    val_buf.push_bits(xor >> trailing, significant_bits as usize);
                    prev_leading = leading;
                    prev_trailing = trailing;
                }
            }
            prev_val_bits = vb;
        }

        Self {
            base_ts: chunk.timestamps[0],
            base_val: chunk.values[0],
            ts_bits: ts_buf.into_vec(),
            val_bits: val_buf.into_vec(),
            rows,
        }
    }

    /// Decode the chunk back to plain column format.
    pub fn decompress(&self) -> ColumnChunk {
        let mut timestamps = Vec::with_capacity(self.rows);
        let mut values = Vec::with_capacity(self.rows);

        // Timestamps
        timestamps.push(self.base_ts);
        let mut reader = BitSlice::<u8, Msb0>::from_slice(&self.ts_bits).expect("bit slice");
        let mut cursor = 0;
        let mut prev_ts = self.base_ts;
        let mut prev_delta = 0i64;
        while timestamps.len() < self.rows {
            if !reader.get(cursor).copied().unwrap_or(false) {
                // control 0 => delta_of_delta = 0
                cursor += 1;
                let delta = prev_delta;
                let ts = prev_ts + delta;
                timestamps.push(ts);
                prev_ts = ts;
            } else {
                cursor += 1;
                let tag = reader[cursor..cursor + 2].load_be::<u8>();
                cursor += 2;
                let (bits, val): (u32, i64) = match tag {
                    0b00 => {
                        let v = reader[cursor..cursor + 12].load_be::<u16>() as u64;
                        cursor += 12;
                        (12, v as i64)
                    }
                    0b01 => {
                        let v = reader[cursor..cursor + 20].load_be::<u32>() as u64;
                        cursor += 20;
                        (20, v as i64)
                    }
                    0b10 => {
                        let v = reader[cursor..cursor + 32].load_be::<u32>() as u64;
                        cursor += 32;
                        (32, v as i64)
                    }
                    _ => {
                        let v = reader[cursor..cursor + 64].load_be::<u64>();
                        cursor += 64;
                        (64, v as i64)
                    }
                };
                // Zigzag decode
                let decoded = ((val >> 1) as i64) ^ (-((val & 1) as i64));
                let delta = prev_delta + decoded;
                let ts = prev_ts + delta;
                prev_ts = ts;
                prev_delta = delta;
                timestamps.push(ts);
                let _ = bits; // silence unused warning
            }
        }

        // Values
        values.push(self.base_val);
        let mut val_reader = BitSlice::<u8, Msb0>::from_slice(&self.val_bits).expect("bit slice");
        let mut val_cursor = 0;
        let mut prev_val_bits = self.base_val.to_bits();
        let mut stored_leading = 64u8;
        let mut stored_trailing = 0u8;

        while values.len() < self.rows {
            let ctrl_zero = !val_reader.get(val_cursor).copied().unwrap_or(false);
            val_cursor += 1;
            if ctrl_zero {
                // value same as previous
                values.push(f64::from_bits(prev_val_bits));
                continue;
            }
            let use_prev_block = !val_reader.get(val_cursor).copied().unwrap_or(false);
            val_cursor += 1;
            let (leading, significant_bits, trailing) = if use_prev_block {
                (stored_leading, 64 - stored_leading as u32 - stored_trailing as u32, stored_trailing)
            } else {
                let leading = val_reader[val_cursor..val_cursor + 6].load_be::<u8>();
                val_cursor += 6;
                let sig_len_minus1 = val_reader[val_cursor..val_cursor + 6].load_be::<u8>();
                val_cursor += 6;
                let significant_bits = (sig_len_minus1 as u32) + 1;
                let trailing = 64 - leading as u32 - significant_bits;
                stored_leading = leading;
                stored_trailing = trailing as u8;
                (leading, significant_bits, trailing as u8)
            };
            let xor_bits = val_reader[val_cursor..val_cursor + significant_bits as usize].load_be::<u64>();
            val_cursor += significant_bits as usize;
            let xor = xor_bits << trailing;
            let curr_bits = prev_val_bits ^ xor;
            values.push(f64::from_bits(curr_bits));
            prev_val_bits = curr_bits;
        }

        ColumnChunk { timestamps, values }
    }
}

/// Time-bucketed index mapping bucket start timestamp to chunk id.
#[derive(Debug, Default)]
pub struct TimeBucketIndex {
    buckets: HashMap<Timestamp, usize>,
    bucket_width: Duration,
}

impl TimeBucketIndex {
    /// Create a new index with the given bucket width.
    pub fn new(bucket_width: Duration) -> Self {
        Self {
            buckets: HashMap::new(),
            bucket_width,
        }
    }

    /// Insert a mapping from timestamp to chunk id.
    pub fn insert(&mut self, ts: Timestamp, chunk_id: usize) {
        let bucket_start = ts - (ts % self.bucket_width.as_nanos() as i64);
        self.buckets.insert(bucket_start, chunk_id);
    }

    /// Locate candidate chunks for the given time range.
    pub fn query(&self, start: Timestamp, end: Timestamp) -> Vec<usize> {
        let mut ids = Vec::new();
        let mut bucket = start - (start % self.bucket_width.as_nanos() as i64);
        while bucket <= end {
            if let Some(&id) = self.buckets.get(&bucket) {
                ids.push(id);
            }
            bucket += self.bucket_width.as_nanos() as i64;
        }
        ids
    }
}

/// Continuous aggregate materializer (simple count, sum, min, max).
#[derive(Debug, Clone)]
pub struct ContinuousAggregate {
    bucket_width: Duration,
    /// Map bucket-start → (count, sum, min, max)
    agg: HashMap<Timestamp, (u64, f64, f64, f64)>,
}

impl ContinuousAggregate {
    /// Create a new materializer with given bucket width.
    pub fn new(bucket_width: Duration) -> Self {
        Self {
            bucket_width,
            agg: HashMap::new(),
        }
    }

    /// Ingest a (timestamp, value) pair updating aggregates.
    pub fn absorb(&mut self, ts: Timestamp, val: f64) {
        let bucket_start = ts - (ts % self.bucket_width.as_nanos() as i64);
        let entry = self.agg.entry(bucket_start).or_insert_with(|| (0, 0.0, val, val));
        entry.0 += 1;
        entry.1 += val;
        if val < entry.2 { entry.2 = val; }
        if val > entry.3 { entry.3 = val; }
    }

    /// Fetch aggregate for a bucket.
    pub fn get(&self, bucket_start: Timestamp) -> Option<&(u64, f64, f64, f64)> {
        self.agg.get(&bucket_start)
    }

    /// Compute average for a bucket, if present.
    pub fn average(&self, bucket_start: Timestamp) -> Option<f64> {
        self.get(bucket_start).map(|(cnt, sum, _, _)| *sum / *cnt as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_compression() {
        let mut chunk = ColumnChunk::new();
        let mut ts = 1_600_000_000_000_000_000i64; // epoch ns
        for i in 0..1000 {
            chunk.append(ts, i as f64 * 0.5);
            ts += 1_000_000; // +1ms
        }
        let compressed = chunk.compress();
        let decompressed = compressed.decompress();
        assert_eq!(chunk.timestamps, decompressed.timestamps);
        assert_eq!(chunk.values, decompressed.values);
    }

    #[test]
    fn bucket_index_query() {
        let mut idx = TimeBucketIndex::new(Duration::from_secs(60));
        idx.insert(0, 1);
        idx.insert(60_000_000_000, 2);
        let res = idx.query(0, 120_000_000_000);
        assert_eq!(res, vec![1, 2]);
    }

    #[test]
    fn continuous_agg() {
        let mut agg = ContinuousAggregate::new(Duration::from_secs(60));
        agg.absorb(0, 1.0);
        agg.absorb(10_000_000_000, 2.0);
        let avg = agg.average(0).unwrap();
        assert!((avg - 1.5).abs() < 1e-6);
    }
} 