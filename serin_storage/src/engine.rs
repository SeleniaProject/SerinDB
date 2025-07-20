use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::{buffer::PageId, PAGE_SIZE};

/// Result type alias for storage operations.
pub type Result<T> = std::result::Result<T, StorageError>;

/// Storage layer errors.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Page not found.
    #[error("page not found: {0:?}")]
    NotFound(PageId),
    /// IO or other underlying error.
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// Abstract storage engine interface for upper layers.
#[async_trait::async_trait]
pub trait StorageEngine: Send + Sync {
    /// Read a page into the provided buffer.
    async fn read_page(&self, page_id: PageId, buf: &mut [u8; PAGE_SIZE]) -> Result<()>;

    /// Write a page from the provided buffer.
    async fn write_page(&self, page_id: PageId, buf: &[u8; PAGE_SIZE]) -> Result<()>;
}

/// In-memory mock storage for testing.
#[derive(Default, Clone)]
pub struct MockStorage {
    pages: Arc<Mutex<HashMap<PageId, Box<[u8; PAGE_SIZE]>>>>,
}

#[async_trait::async_trait]
impl StorageEngine for MockStorage {
    async fn read_page(&self, page_id: PageId, buf: &mut [u8; PAGE_SIZE]) -> Result<()> {
        let pages = self.pages.lock().unwrap();
        if let Some(page) = pages.get(&page_id) {
            buf.copy_from_slice(page);
            Ok(())
        } else {
            Err(StorageError::NotFound(page_id))
        }
    }

    async fn write_page(&self, page_id: PageId, buf: &[u8; PAGE_SIZE]) -> Result<()> {
        let mut pages = self.pages.lock().unwrap();
        pages.insert(page_id, Box::new(*buf));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PAGE_SIZE;

    #[tokio::test]
    async fn mock_storage_rw() {
        let storage = MockStorage::default();
        let page_id = PageId(42);
        let data = [7u8; PAGE_SIZE];
        storage.write_page(page_id, &data).await.unwrap();
        let mut read_buf = [0u8; PAGE_SIZE];
        storage.read_page(page_id, &mut read_buf).await.unwrap();
        assert_eq!(data, read_buf);
    }
} 