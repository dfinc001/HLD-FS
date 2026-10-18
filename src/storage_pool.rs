// src/storage_pool.rs
use std::sync::atomic::{AtomicU64, Ordering};

pub struct StoragePool {
    total_capacity: u64,
    used_bytes: AtomicU64,
    cold_alloc_head: AtomicU64,
}

impl StoragePool {
    pub fn new(total_capacity: u64) -> Self {
        let cold_start = total_capacity / 2;
        Self {
            total_capacity,
            used_bytes: AtomicU64::new(0),
            cold_alloc_head: AtomicU64::new(cold_start),
        }
    }

    pub fn is_safety_valve_tripped(&self) -> bool {
        let used = self.used_bytes.load(Ordering::Relaxed);
        (used as f64 / self.total_capacity as f64) >= 0.75
    }

    pub fn available_space(&self) -> u64 {
        let used = self.used_bytes.load(Ordering::Relaxed);
        self.total_capacity.saturating_sub(used)
    }

    pub fn add_used(&self, bytes: u64) {
        self.used_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Allocates space in the cold storage pool region
    pub fn allocate_cold_space(&self, size: u64) -> Result<u64, crate::HldFsError> {
        let offset = self.cold_alloc_head.fetch_add(size, Ordering::SeqCst);
        if offset + size > self.total_capacity {
            return Err(crate::HldFsError::StorageFull);
        }
        Ok(offset)
    }
}
