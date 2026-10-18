use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use crate::disk::DiskManager;
use crate::storage_pool::StoragePool;
use crate::{Result, HldFsError};

pub struct LogTier {
    disk: Arc<DiskManager>,
    storage_pool: Arc<StoragePool>,
    // Tracks the current append head offset in the log tier region of the disk
    current_head: AtomicU64,
    max_log_capacity: u64,
}

impl LogTier {
    /// Initializes the log tier, pointing the initial write head to a given byte offset.
    pub fn new(
        disk: Arc<DiskManager>,
        storage_pool: Arc<StoragePool>,
        start_offset: u64,
        max_log_capacity: u64,
    ) -> Self {
        Self {
            disk,
            storage_pool,
            current_head: AtomicU64::new(start_offset),
            max_log_capacity,
        }
    }

    /// Appends incoming hot write data sequentially to the log.
    /// Returns the starting log offset where the data was written.
    pub fn append(&self, data: &[u8]) -> Result<u64> {
        let data_len = data.len() as u64;

        // 1. Check the 75% utilization safety valve before letting new writes proceed
        if self.storage_pool.is_safety_valve_tripped() {
            return Err(HldFsError::StorageFull);
        }

        // 2. Atomically reserve space in the append-only log
        // Fetch-and-add returns the old head value, then advances it safely for concurrent writers
        let write_offset = self.current_head.fetch_add(data_len, Ordering::SeqCst);

        // Ensure we don't overflow the designated log partition space
        if write_offset + data_len > self.disk.capacity() {
            return Err(HldFsError::StorageFull);
        }

        // 3. Perform the blazing-fast sequential write via DiskManager
        self.disk.write_at(write_offset, data)?;

        // 4. Update global storage pool used bytes tracking
        // (Assuming each append adds uniquely to the active footprint until compaction)
        // Note: In a real implementation, you might track this dynamically via your allocator.
        
        Ok(write_offset)
    }

    /// Reads hot data directly from a given log offset (used by FUSE read handlers for active files).
    pub fn read_log(&self, offset: u64, buf: &mut [u8]) -> Result<()> {
        self.disk.read_at(offset, buf)?;
        Ok(())
    }

    pub fn current_head(&self) -> u64 {
        self.current_head.load(Ordering::Relaxed)
    }
}
