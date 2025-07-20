use std::collections::{HashMap, VecDeque};
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

use crate::{compute_checksum, PAGE_SIZE};

/// Logical identifier of a page (tablespace, file, block number).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageId(pub u64);

/// In-memory buffer frame containing a page.
#[derive(Debug)]
struct BufferFrame {
    page_id: PageId,
    data: Box<[u8; PAGE_SIZE]>,
    pin_count: u32,
    is_dirty: bool,
    clock_ref: bool,
}

impl BufferFrame {
    fn new(page_id: PageId) -> Self {
        Self {
            page_id,
            data: Box::new([0u8; PAGE_SIZE]),
            pin_count: 0,
            is_dirty: false,
            clock_ref: false,
        }
    }
}

/// Adaptive 2Q buffer pool.
pub struct BufferPool {
    /// Maximum number of pages in the cache.
    capacity: usize,
    /// Main buffer list (Am) – LRU.
    am: VecDeque<PageId>,
    /// Recent-in list (A1in) – FIFO.
    a1_in: VecDeque<PageId>,
    /// Recent-out ghost list (A1out) – stores page ids only.
    a1_out: VecDeque<PageId>,
    /// Mapping from PageId to frame.
    frames: HashMap<PageId, Arc<Mutex<BufferFrame>>>,
}

impl BufferPool {
    /// Create a new buffer pool with given capacity (in pages).
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            am: VecDeque::new(),
            a1_in: VecDeque::new(),
            a1_out: VecDeque::new(),
            frames: HashMap::new(),
        }
    }

    /// Fetch a page into the buffer pool, returning a handle to its frame.
    pub fn fetch_page(&mut self, page_id: PageId) -> Arc<Mutex<BufferFrame>> {
        if let Some(frame) = self.frames.get(&page_id) {
            // Hit in buffer – update lists.
            self.touch(page_id);
            return Arc::clone(frame);
        }

        // Miss – need to allocate.
        self.ensure_capacity();

        let frame = Arc::new(Mutex::new(BufferFrame::new(page_id)));
        self.frames.insert(page_id, Arc::clone(&frame));
        self.a1_in.push_front(page_id);
        frame
    }

    /// Touch a page id when it is accessed.
    fn touch(&mut self, page_id: PageId) {
        if let Some(pos) = self.am.iter().position(|&id| id == page_id) {
            // Move to front (MRU)
            self.am.remove(pos);
            self.am.push_front(page_id);
        } else if let Some(pos) = self.a1_in.iter().position(|&id| id == page_id) {
            // Promote to Am
            self.a1_in.remove(pos);
            self.am.push_front(page_id);
        }
    }

    /// Ensure there is space for a new page by evicting if necessary.
    fn ensure_capacity(&mut self) {
        if self.frames.len() < self.capacity {
            return;
        }
        // Eviction policy based on 2Q.
        if !self.a1_in.is_empty() {
            if let Some(old) = self.a1_in.pop_back() {
                self.evict(old);
                self.a1_out.push_front(old);
                return;
            }
        }
        // Otherwise evict from Am using LRU (could implement CLOCK)
        if let Some(old) = self.am.pop_back() {
            self.evict(old);
        }
    }

    fn evict(&mut self, page_id: PageId) {
        self.frames.remove(&page_id);
        // In production, would flush dirty page to disk.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_fetch_and_evict() {
        let mut pool = BufferPool::new(2);
        let p1 = pool.fetch_page(PageId(1));
        let p2 = pool.fetch_page(PageId(2));
        // Third fetch triggers eviction.
        let _p3 = pool.fetch_page(PageId(3));
        assert_eq!(pool.frames.len(), 2);
    }
} 