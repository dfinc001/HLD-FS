// src/storage_pool.rs
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use crate::disk::DiskManager;
use crate::error::HldFsError;

pub struct StoragePool {
    total_capacity: AtomicU64,
    used_bytes: AtomicU64,
    cold_alloc_head: AtomicU64,
    disk: Arc<DiskManager>,
    disk_path: String,
    chunk_size: u64,
}

impl StoragePool {
    pub fn new(initial_capacity: u64, disk: Arc<DiskManager>, disk_path: String) -> Self {
        let cold_start = initial_capacity / 2;
        Self {
            total_capacity: AtomicU64::new(initial_capacity),
            used_bytes: AtomicU64::new(0),
            cold_alloc_head: AtomicU64::new(cold_start),
            disk,
            disk_path,
            chunk_size: 1024 * 1024 * 1024, // 1 GB chunks
        }
    }

    pub fn total_capacity(&self) -> u64 {
        self.total_capacity.load(Ordering::Relaxed)
    }

    pub fn used_bytes(&self) -> u64 {
        self.used_bytes.load(Ordering::Relaxed)
    }

    pub fn available_space(&self) -> u64 {
        let total = self.total_capacity.load(Ordering::Relaxed);
        let used = self.used_bytes.load(Ordering::Relaxed);
        total.saturating_sub(used)
    }

    pub fn is_safety_valve_tripped(&self) -> bool {
        let total = self.total_capacity.load(Ordering::Relaxed);
        let used = self.used_bytes.load(Ordering::Relaxed);
        (used as f64 / total as f64) >= 0.75
    }

    pub fn add_used(&self, bytes: u64) {
        self.used_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    fn check_host_free_space(&self) -> std::io::Result<u64> {
        let path = std::path::Path::new(&self.disk_path);
        let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let c_path = std::ffi::CString::new(parent.to_string_lossy().as_bytes())?;
        let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
        let res = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
        if res != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(stat.f_bavail as u64 * stat.f_frsize as u64)
    }

    pub fn ensure_capacity(&self, required_offset: u64) -> Result<(), HldFsError> {
        let mut current_cap = self.total_capacity.load(Ordering::Relaxed);
        while required_offset > current_cap {
            if let Ok(host_free) = self.check_host_free_space() {
                if host_free < self.chunk_size {
                    return Err(HldFsError::StorageFull);
                }
            }

            let new_cap = current_cap + self.chunk_size;
            self.disk.resize(new_cap).map_err(|_| HldFsError::StorageFull)?;
            self.total_capacity.store(new_cap, Ordering::SeqCst);
            current_cap = new_cap;
            println!("[HLD-FS] StoragePool auto-expanded by 1GB. New capacity: {} bytes", new_cap);
        }
        Ok(())
    }

    pub fn allocate_cold_space(&self, size: u64) -> Result<u64, HldFsError> {
        let offset = self.cold_alloc_head.fetch_add(size, Ordering::SeqCst);
        self.ensure_capacity(offset + size)?;
        Ok(offset)
    }
}
