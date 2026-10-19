use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColdFileEntry {
    pub chunk_hashes: Vec<[u8; 32]>, // BLAKE3 cryptographic fingerprints for each 128KB chunk
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub physical_offset: u64,
    pub ref_count: u32,
}

pub struct ColdMap {
    file_chunks: RwLock<HashMap<u64, ColdFileEntry>>,
    chunk_index: RwLock<HashMap<[u8; 32], ChunkMetadata>>,
}

impl ColdMap {
    pub fn new() -> Self {
        Self {
            file_chunks: RwLock::new(HashMap::new()),
            chunk_index: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert_file_chunks(&self, inode: u64, chunk_hashes: Vec<[u8; 32]>) {
        let mut guard = self.file_chunks.write();
        guard.insert(inode, ColdFileEntry { chunk_hashes });
    }

    pub fn get_file_chunks(&self, inode: u64) -> Option<Vec<[u8; 32]>> {
        let guard = self.file_chunks.read();
        guard.get(&inode).map(|entry| entry.chunk_hashes.clone())
    }

    pub fn register_chunk(&self, hash: [u8; 32], physical_offset: u64) -> bool {
        let mut index = self.chunk_index.write();
        if let Some(meta) = index.get_mut(&hash) {
            meta.ref_count += 1;
            false
        } else {
            index.insert(
                hash,
                ChunkMetadata {
                    physical_offset,
                    ref_count: 1,
                },
            );
            true
        }
    }

    pub fn get_chunk_location(&self, hash: &[u8; 32]) -> Option<u64> {
        let index = self.chunk_index.read();
        index.get(hash).map(|meta| meta.physical_offset)
    }

    pub fn deref_chunk(&self, hash: &[u8; 32]) -> Option<u32> {
        let mut index = self.chunk_index.write();
        if let Some(meta) = index.get_mut(hash) {
            meta.ref_count = meta.ref_count.saturating_sub(1);
            let count = meta.ref_count;
            if count == 0 {
                index.remove(hash);
            }
            Some(count)
        } else {
            None
        }
    }

    pub fn snapshot_file_chunks(&self) -> HashMap<u64, ColdFileEntry> {
        self.file_chunks.read().clone()
    }

    pub fn snapshot_chunk_index(&self) -> HashMap<[u8; 32], ChunkMetadata> {
        self.chunk_index.read().clone()
    }

    pub fn restore(&self, file_chunks: HashMap<u64, ColdFileEntry>, chunk_index: HashMap<[u8; 32], ChunkMetadata>) {
        *self.file_chunks.write() = file_chunks;
        *self.chunk_index.write() = chunk_index;
    }
}
