// src/metadata/mod.rs
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

#[derive(Clone)]
pub enum FileState {
    Hot { log_offset: u64, length: u64 },
    Cold { chunk_hashes: Vec<[u8; 32]> },
}

pub struct DualMapMetadata {
    // Maps Inode -> File State (Hot or Cold)
    map: Arc<RwLock<HashMap<u64, FileState>>>,
}

impl DualMapMetadata {
    pub fn new() -> Self {
        Self {
            map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_state(&self, inode: u64) -> Option<FileState> {
        let guard = self.map.read();
        guard.get(&inode).cloned()
    }

    pub fn transition_to_cold(&self, inode: u64, new_chunks: Vec<[u8; 32]>) {
        let mut guard = self.map.write();
        guard.insert(inode, FileState::Cold { chunk_hashes: new_chunks });
    }
}
