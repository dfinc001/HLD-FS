use std::collections::HashMap;
use parking_lot::RwLock;

#[derive(Debug, Clone, Copy)]
pub struct HotFileEntry {
    pub log_offset: u64,
    pub length: u64,
}

pub struct HotMap {
    entries: RwLock<HashMap<u64, HotFileEntry>>,
}

impl HotMap {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Inserts or updates a hot file entry mapping an inode to its current log offset and length.
    pub fn insert(&self, inode: u64, log_offset: u64, length: u64) {
        let mut guard = self.entries.write();
        guard.insert(inode, HotFileEntry { log_offset, length });
    }

    /// Retrieves the log coordinates for a hot file inode.
    pub fn get(&self, inode: u64) -> Option<HotFileEntry> {
        let guard = self.entries.read();
        guard.get(&inode).copied()
    }

    /// Removes a file from the hot map (typically called when migrating a file to the cold tier).
    pub fn remove(&self, inode: u64) -> Option<HotFileEntry> {
        let mut guard = self.entries.write();
        guard.remove(&inode)
    }

    pub fn len(&self) -> usize {
        let guard = self.entries.read();
        guard.len()
    }

    pub fn is_empty(&self) -> bool {
        let guard = self.entries.read();
        guard.is_empty()
    }
}
