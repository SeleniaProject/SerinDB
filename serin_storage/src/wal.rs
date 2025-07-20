use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;

/// WAL record header: length of payload.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct WalHeader {
    len: u32,
    ts: i64, // unix timestamp ns
}

/// Writer for write-ahead log with simple group commit.
#[derive(Debug)]
pub struct WalWriter {
    inner: Arc<Mutex<File>>,
    buffer: Vec<u8>,
    buffer_limit: usize,
}

impl WalWriter {
    /// Open WAL file (create if not exists) at given path.
    pub fn open<P: AsRef<Path>>(path: P, buffer_limit: usize) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(path)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(file)),
            buffer: Vec::with_capacity(buffer_limit),
            buffer_limit,
        })
    }

    /// Append a binary payload to WAL.
    pub fn append(&mut self, payload: &[u8]) -> std::io::Result<()> {
        let hdr = WalHeader {
            len: payload.len() as u32,
            ts: OffsetDateTime::now_utc().unix_timestamp_nanos(),
        };
        let hdr_bytes = unsafe {
            std::slice::from_raw_parts(
                &hdr as *const WalHeader as *const u8,
                std::mem::size_of::<WalHeader>(),
            )
        };
        self.buffer.extend_from_slice(hdr_bytes);
        self.buffer.extend_from_slice(payload);

        if self.buffer.len() >= self.buffer_limit {
            self.flush()?;
        }
        Ok(())
    }

    /// Flush buffer to disk with fsync (group commit).
    pub fn flush(&mut self) -> std::io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let mut file = self.inner.lock().unwrap();
        file.write_all(&self.buffer)?;
        file.sync_data()?;
        self.buffer.clear();
        Ok(())
    }
}

impl Drop for WalWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

/// Iterate over WAL records from a file path.
pub fn iter_log<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<Vec<u8>>> {
    let mut file = File::open(path)?;
    let mut records = Vec::new();
    loop {
        let mut hdr_buf = [0u8; std::mem::size_of::<WalHeader>()];
        if let Err(e) = file.read_exact(&mut hdr_buf) {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                break;
            } else {
                return Err(e);
            }
        }
        let hdr: WalHeader = unsafe { std::ptr::read(hdr_buf.as_ptr() as *const _) };
        let mut payload = vec![0u8; hdr.len as usize];
        file.read_exact(&mut payload)?;
        records.push(payload);
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn wal_append_and_replay() {
        let path = "./test_wal.bin";
        let _ = fs::remove_file(path);
        {
            let mut writer = WalWriter::open(path, 128).unwrap();
            writer.append(b"record1").unwrap();
            writer.append(b"record2").unwrap();
            writer.flush().unwrap();
        }
        let recs = iter_log(path).unwrap();
        assert_eq!(recs, vec![b"record1".to_vec(), b"record2".to_vec()]);
        fs::remove_file(path).unwrap();
    }
} 