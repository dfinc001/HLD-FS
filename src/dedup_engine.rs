// src/dedup_engine.rs
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use crate::storage_pool::StoragePool;
use crate::metadata::DualMapMetadata;
use crate::disk::DiskManager;

const CHUNK_SIZE: usize = 128 * 1024; // 128KB chunks

pub struct DedupWorker {
    storage_pool: Arc<StoragePool>,
    metadata: Arc<DualMapMetadata>,
    disk: Arc<DiskManager>,
}

impl DedupWorker {
    pub fn new(
        storage_pool: Arc<StoragePool>,
        metadata: Arc<DualMapMetadata>,
        disk: Arc<DiskManager>,
    ) -> Self {
        Self { storage_pool, metadata, disk }
    }

    pub async fn run_background_loop(&self) {
        loop {
            // 1. Check the 75% safety valve before doing any compaction work
            if self.storage_pool.is_safety_valve_tripped() {
                eprintln!("Safety valve active (>= 75% utilization). Pausing background deduplication.");
                sleep(Duration::from_secs(10)).await;
                continue;
            }

            // 2. Snapshot hot entries to process migration
            let hot_entries = self.metadata.hot_map.snapshot();
            for (inode, entry) in hot_entries {
                if let Err(e) = self.migrate_file(inode, entry.log_offset, entry.length) {
                    eprintln!("Failed to migrate inode {} to cold tier: {:?}", inode, e);
                }
            }

            sleep(Duration::from_secs(5)).await;
        }
    }

    fn migrate_file(&self, inode: u64, log_offset: u64, length: u64) -> crate::Result<()> {
        // Read raw hot data from the log tier
        let mut data = vec![0u8; length as usize];
        self.disk.read_at(log_offset, &mut data)?;

        let mut chunk_hashes = Vec::new();

        // Split into 128KB chunks and process BLAKE3 deduplication
        for chunk_data in data.chunks(CHUNK_SIZE) {
            let hash: [u8; 32] = *blake3::hash(chunk_data).as_bytes();

            // Check if chunk already exists in cold map, otherwise allocate and store
            if let Some(_existing_offset) = self.metadata.cold_map.get_chunk_location(&hash) {
                // Duplicate chunk found: Increment ref count via registration
                self.metadata.cold_map.register_chunk(hash, 0); // 0 means already stored
            } else {
                // Unique chunk: Allocate physical space and write to disk
                let physical_offset = self.storage_pool.allocate_cold_space(chunk_data.len() as u64)?;
                self.disk.write_at(physical_offset, chunk_data)?;
                self.metadata.cold_map.register_chunk(hash, physical_offset);
                self.storage_pool.add_used(chunk_data.len() as u64);
            }

            chunk_hashes.push(hash);
        }

        // Bind chunk hashes to cold map and remove from hot map
        self.metadata.cold_map.insert_file_chunks(inode, chunk_hashes);
        self.metadata.hot_map.remove(inode);

        println!("Successfully migrated inode {} ({} bytes) to cold deduplicated tier.", inode, length);
        Ok(())
    }
}
