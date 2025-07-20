use std::io::Result;
use std::path::Path;

/// Read a page at given offset using tokio-uring.
#[cfg(feature = "uring")]
pub async fn pread<P: AsRef<Path>>(path: P, offset: u64, buf: &mut [u8]) -> Result<usize> {
    use tokio_uring::fs::File;
    let file = File::open(path).await?;
    let (res, _buf) = file.read_at(buf, offset).await;
    res
}

/// Write a page at given offset using tokio-uring.
#[cfg(feature = "uring")]
pub async fn pwrite<P: AsRef<Path>>(path: P, offset: u64, data: &[u8]) -> Result<usize> {
    use tokio_uring::fs::OpenOptions;
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(path)
        .await?;
    let (res, _) = file.write_at(data, offset).await;
    res
}

#[cfg(all(test, feature = "uring"))]
mod tests {
    use super::*;
    use crate::PAGE_SIZE;
    use std::fs;

    #[tokio::test]
    async fn uring_read_write() {
        let path = "./test_page.bin";
        let _ = fs::remove_file(path);
        let write_buf = vec![42u8; PAGE_SIZE];
        pwrite(path, 0, &write_buf).await.unwrap();
        let mut read_buf = vec![0u8; PAGE_SIZE];
        pread(path, 0, &mut read_buf).await.unwrap();
        assert_eq!(write_buf, read_buf);
        fs::remove_file(path).unwrap();
    }
} 