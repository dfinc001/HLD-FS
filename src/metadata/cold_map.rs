use std::collections::HashMap;
use parking_lot::RwLock;

#[derive(Debug, Clone)]
pub struct ColdFileEntry {
    pub chunk_hashes: Vec<[u8; 32]>, // BLAKE3 cryptographic fingerprints for each 128KB chunk
}

#[derive(Debug, Clone, Copy)]
pub struct ChunkMetadata {
    pub physical_offset: u64,
    pub ref_count: u32,
}

pub struct ColdMap {
    // Maps file Inode -> ordered sequence of 128KB chunk hashes
    file_chunks: RwLock<HashMap<u64, ColdFileEntry>>,
    
    // Global chunk index: Maps BLAKE3 Hash -> Physical storage offset and reference count (for deduplication)
    chunk_index: RwLock<HashMap<[u8; 32], ChunkMetadata>>,
}

impl ColdMap {
    pub fn new() -> Self {
        Self {
            file_chunks: RwLock::new(HashMap::new()),
            chunk_index: RwLock::new(HashMap::new()),
        }
    }

    /// Binds an inode to its immutable sequence of BLAKE3 chunk hashes.
    pub fn insert_file_chunks(&self, inode: u64, chunk_hashes: Vec<[u8; 32]>) {
        let mut guard = self.file_chunks.write();
        guard.insert(inode, ColdFileEntry { chunk_hashes });
    }

    /// Retrieves the ordered chunk hashes for a cold file inode.
    pub fn get_file_chunks(&self, inode: u64) -> Option<Vec<[u8; 32]>> {
        let guard = self.file_chunks.read();
        guard.get(&inode).map(|entry| entry.chunk_hashes.clone())
    }

    /// Registers a chunk hash. If the chunk already exists, increments its reference count (deduplication).
    /// Returns `true` if this is a brand-new unique chunk requiring disk storage, or `false` if it's a duplicate.
    pub fn register_chunk(&self, hash: [u8; 32], physical_offset: u64) -> bool {
        let mut index = self.chunk_index.write();
        if let Some(meta) = index.get_mut(&hash) {
            meta.ref_count += 1;
            false // Duplicate chunk found! Storage saved.
        } else {
            index.insert(
                hash,
                ChunkMetadata {
                    physical_offset,
                    ref_count: 1,
                },
            );
            true // Unique chunk stored for the first time.
        }
    }

    /// Looks up the physical disk offset of a chunk using its BLAKE3 hash.
    pub fn get_chunk_location(&self, hash: &[u8; 32]) -> Option<u64> {
        let index = self.chunk_index.read();
        index.get(hash).map(|meta| meta.physical_offset)
    }

    /// Decrements a chunk's reference count during garbage collection.
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
}
