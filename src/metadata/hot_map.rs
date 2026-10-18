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

    pub fn insert(&self, inode: u64, log_offset: u64, length: u64) {
        let mut guard = self.entries.write();
        guard.insert(inode, HotFileEntry { log_offset, length });
    }

    pub fn get(&self, inode: u64) -> Option<HotFileEntry> {
        let guard = self.entries.read();
        guard.get(&inode).copied()
    }

    pub fn remove(&self, inode: u64) -> Option<HotFileEntry> {
        let mut guard = self.entries.write();
        guard.remove(&inode)
    }

    pub fn snapshot(&self) -> Vec<(u64, HotFileEntry)> {
        let guard = self.entries.read();
        guard.iter().map(|(&k, &v)| (k, v)).collect()
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
