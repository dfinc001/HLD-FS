// src/storage_pool.rs
pub struct StoragePool {
    total_capacity: u64,
    used_bytes: std::sync::atomic::AtomicU64,
}

impl StoragePool {
    pub fn new(total_capacity: u64) -> Self {
        Self {
            total_capacity,
            used_bytes: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Checks if physical storage has crossed the 75% utilization safety valve.
    pub fn is_safety_valve_tripped(&self) -> bool {
        let used = self.used_bytes.load(std::sync::atomic::Ordering::Relaxed);
        let utilization = used as f64 / self.total_capacity as f64;
        utilization >= 0.75
    }

    pub fn available_space(&self) -> u64 {
        let used = self.used_bytes.load(std::sync::atomic::Ordering::Relaxed);
        self.total_capacity.saturating_sub(used)
    }

    pub fn add_used(&self, bytes: u64) {
        self.used_bytes.fetch_add(bytes, std::sync::atomic::Ordering::Relaxed);
    }
}
