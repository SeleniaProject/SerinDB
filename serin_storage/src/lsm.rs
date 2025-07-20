//! Log-Structured Merge-Tree Level 0-1 implementation.
//! This is an initial, single-threaded version that focuses on correctness
//! rather than full production scalability. It is nonetheless designed so
//! that future concurrency and compaction work can be added without breaking
//! the API.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use skiplist::SkipMap;

/// The in-memory data structure that buffers recent writes before they are
/// flushed to an on-disk SSTable. A lock-free skiplist gives us O(log N)
/// inserts and searches while preserving sorted order for fast flushes.
#[derive(Debug, Default)]
pub struct MemTable {
    inner: Arc<SkipMap<Vec<u8>, Vec<u8>>>,
    /// Approximate size in bytes. We track this so we know when to flush.
    size_bytes: Arc<RwLock<usize>>,
}

impl MemTable {
    /// Create a new, empty MemTable.
    pub fn new() -> Self {
        Self { inner: Arc::new(SkipMap::new()), size_bytes: Arc::new(RwLock::new(0)) }
    }

    /// Insert or update a key/value pair.
    pub fn insert(&self, key: Vec<u8>, value: Vec<u8>) {
        let delta = key.len() + value.len();
        self.inner.insert(key, value);
        let mut sz = self.size_bytes.write().unwrap();
        *sz += delta;
    }

    /// Retrieve the value for a key if it is still resident in memory.
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.inner.get(key).map(|entry| entry.value().clone())
    }

    /// Return an iterator over the items in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = (Vec<u8>, Vec<u8>)> + '_ {
        self.inner.iter().map(|entry| (entry.key().clone(), entry.value().clone()))
    }

    /// Current size in bytes.
    pub fn size(&self) -> usize { *self.size_bytes.read().unwrap() }

    /// Clear the memtable after it has been flushed.
    fn clear(&self) {
        self.inner.clear();
        *self.size_bytes.write().unwrap() = 0;
    }
}

/// SSTable file footer magic value for format validation.
const FOOTER_MAGIC: u32 = 0x534B_5950; // "SKYP" – arbitrary four-byte tag

/// A simple, immutable Sorted String Table file.
pub struct SsTableWriter {
    path: PathBuf,
}

impl SsTableWriter {
    /// Flush a memtable into a brand-new SSTable file. The memtable is *not* cleared;
    /// the caller is responsible for doing so if the flush succeeds.
    pub fn flush_to_path(mem: &MemTable, dir: &Path, file_id: u64) -> std::io::Result<Self> {
        let file_name = format!("{:020}.sst", file_id);
        let path = dir.join(file_name);
        let mut file = OpenOptions::new().create(true).write(true).truncate(true).open(&path)?;

        // Write key/value pairs in sorted order (skipmap already sorted).
        // Record the offset of each entry so we can build a footer index.
        let mut index: Vec<(Vec<u8>, u64)> = Vec::with_capacity(mem.inner.len());

        for (key, value) in mem.iter() {
            let offset = file.stream_position()?;
            // Entry format: [key_len: u32][val_len: u32][key][val]
            let key_len = key.len() as u32;
            let val_len = value.len() as u32;
            file.write_all(&key_len.to_le_bytes())?;
            file.write_all(&val_len.to_le_bytes())?;
            file.write_all(&key)?;
            file.write_all(&value)?;
            index.push((key, offset));
        }

        // Write the index – sequence of (key_len, key, offset)
        let index_offset = file.stream_position()?;
        for (key, offset) in &index {
            let key_len = key.len() as u32;
            file.write_all(&key_len.to_le_bytes())?;
            file.write_all(key)?;
            file.write_all(&offset.to_le_bytes())?; // u64 little-endian
        }

        // Write footer: [index_offset: u64][magic: u32]
        file.write_all(&index_offset.to_le_bytes())?;
        file.write_all(&FOOTER_MAGIC.to_le_bytes())?;
        file.flush()?;
        Ok(Self { path })
    }

    /// Return the path of the written SSTable.
    pub fn path(&self) -> &Path { &self.path }
}

/// Reader for an SSTable that loads a sparse in-memory index to enable efficient point lookups.
pub struct SsTableReader {
    file: File,
    index: HashMap<Vec<u8>, u64>,
}

impl SsTableReader {
    /// Open an existing SSTable and read its footer + index into memory.
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let mut file = OpenOptions::new().read(true).open(path)?;
        let file_len = file.metadata()?.len();
        if file_len < 12 { // index_offset (8) + magic (4)
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "SSTable too small"));
        }
        file.seek(SeekFrom::End(-12))?;
        let mut buf8 = [0u8; 8];
        let mut buf4 = [0u8; 4];
        file.read_exact(&mut buf8)?;
        file.read_exact(&mut buf4)?;
        let index_offset = u64::from_le_bytes(buf8);
        let magic = u32::from_le_bytes(buf4);
        if magic != FOOTER_MAGIC {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Bad SSTable magic"));
        }

        // Load the index map.
        let mut index = HashMap::new();
        file.seek(SeekFrom::Start(index_offset))?;
        while (file.stream_position()? as u64) < file_len - 12 {
            let mut key_len_buf = [0u8; 4];
            file.read_exact(&mut key_len_buf)?;
            let key_len = u32::from_le_bytes(key_len_buf) as usize;
            let mut key = vec![0u8; key_len];
            file.read_exact(&mut key)?;
            let mut off_buf = [0u8; 8];
            file.read_exact(&mut off_buf)?;
            let offset = u64::from_le_bytes(off_buf);
            index.insert(key, offset);
        }
        Ok(Self { file, index })
    }

    /// Get a value for the key, if present.
    pub fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        let &offset = self.index.get(key)?;
        if self.file.seek(SeekFrom::Start(offset)).is_err() {
            return None;
        }
        let mut len_buf = [0u8; 4];
        // key_len
        if self.file.read_exact(&mut len_buf).is_err() {
            return None;
        }
        let key_len = u32::from_le_bytes(len_buf) as usize;
        // val_len
        if self.file.read_exact(&mut len_buf).is_err() {
            return None;
        }
        let val_len = u32::from_le_bytes(len_buf) as usize;
        // skip key bytes
        if self.file.seek(SeekFrom::Current(key_len as i64)).is_err() {
            return None;
        }
        let mut val = vec![0u8; val_len];
        if self.file.read_exact(&mut val).is_err() {
            return None;
        }
        Some(val)
    }
}

/// A minimal, single-threaded LSM tree covering level 0 and level 1 with size-based flushes.
/// It does not yet implement compaction or deletion tombstones.
#[derive(Debug)]
pub struct LsmTree {
    mem: MemTable,
    /// Ordered newest-to-oldest so we search recent tables first (shadowing older entries).
    sstables: Vec<SsTableReader>,
    dir: PathBuf,
    next_file_id: u64,
    /// Flush threshold in bytes.
    flush_threshold: usize,
}

impl LsmTree {
    /// Create an LSM tree rooted at the given directory. If the directory already contains
    /// SSTables, they are loaded in descending file id order.
    pub fn open_or_create(dir: impl AsRef<Path>, flush_threshold: usize) -> std::io::Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;
        let mut entries: Vec<_> = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "sst").unwrap_or(false))
            .collect();
        entries.sort_by_key(|e| e.path()); // ascending
        let mut sstables = Vec::new();
        let mut next_file_id = 0;
        for entry in entries.into_iter().rev() { // newest first
            if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                if let Ok(id) = stem.parse::<u64>() {
                    next_file_id = next_file_id.max(id + 1);
                }
            }
            if let Ok(reader) = SsTableReader::open(&entry.path()) {
                sstables.push(reader);
            }
        }
        Ok(Self { mem: MemTable::new(), sstables, dir, next_file_id, flush_threshold })
    }

    /// Insert or update a key/value pair.
    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> std::io::Result<()> {
        self.mem.insert(key, value);
        if self.mem.size() >= self.flush_threshold { self.flush()?; }
        Ok(())
    }

    /// Retrieve a value for the key if it exists in the memtable or any SSTable.
    pub fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        if let Some(val) = self.mem.get(key) { return Some(val); }
        for table in &mut self.sstables { if let Some(v) = table.get(key) { return Some(v); } }
        None
    }

    /// Flush the memtable to a new level-0 SSTable on disk.
    pub fn flush(&mut self) -> std::io::Result<()> {
        if self.mem.size() == 0 { return Ok(()); }
        let writer = SsTableWriter::flush_to_path(&self.mem, &self.dir, self.next_file_id)?;
        self.next_file_id += 1;
        self.mem.clear();
        // Load the table we just wrote so that it participates in reads immediately.
        let reader = SsTableReader::open(writer.path())?;
        self.sstables.insert(0, reader); // newest first
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn memtable_basic() {
        let mem = MemTable::new();
        mem.insert(b"key1".to_vec(), b"val1".to_vec());
        assert_eq!(mem.get(b"key1"), Some(b"val1".to_vec()));
        assert_eq!(mem.get(b"key2"), None);
    }

    #[test]
    fn sstable_roundtrip() {
        let dir = TempDir::new().unwrap();
        let mem = MemTable::new();
        mem.insert(b"a".to_vec(), b"1".to_vec());
        mem.insert(b"b".to_vec(), b"2".to_vec());
        let writer = SsTableWriter::flush_to_path(&mem, dir.path(), 0).unwrap();
        let mut reader = SsTableReader::open(writer.path()).unwrap();
        assert_eq!(reader.get(b"a"), Some(b"1".to_vec()));
        assert_eq!(reader.get(b"b"), Some(b"2".to_vec()));
        assert_eq!(reader.get(b"c"), None);
    }

    #[test]
    fn lsm_tree_put_get() {
        let tmp = TempDir::new().unwrap();
        let mut tree = LsmTree::open_or_create(tmp.path(), 1024).unwrap();
        tree.put(b"hello".to_vec(), b"world".to_vec()).unwrap();
        assert_eq!(tree.get(b"hello"), Some(b"world".to_vec()));
        // Force flush.
        tree.flush().unwrap();
        assert_eq!(tree.get(b"hello"), Some(b"world".to_vec()));
    }
} 