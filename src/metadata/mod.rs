use std::sync::Arc;
use std::collections::HashMap;
use std::path::Path;
use serde::{Serialize, Deserialize};

pub mod hot_map;
pub mod cold_map;

pub use hot_map::HotMap;
pub use cold_map::ColdMap;

#[derive(Serialize, Deserialize)]
struct CheckpointData {
    hot_entries: HashMap<u64, hot_map::HotFileEntry>,
    file_chunks: HashMap<u64, cold_map::ColdFileEntry>,
    chunk_index: HashMap<[u8; 32], cold_map::ChunkMetadata>,
}

pub struct DualMapMetadata {
    pub hot_map: Arc<HotMap>,
    pub cold_map: Arc<ColdMap>,
}

impl DualMapMetadata {
    pub fn new() -> Self {
        Self {
            hot_map: Arc::new(HotMap::new()),
            cold_map: Arc::new(ColdMap::new()),
        }
    }

    pub fn save_to_disk<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let hot_entries = self.hot_map.snapshot().into_iter().collect();
        let file_chunks = self.cold_map.snapshot_file_chunks();
        let chunk_index = self.cold_map.snapshot_chunk_index();

        let checkpoint = CheckpointData {
            hot_entries,
            file_chunks,
            chunk_index,
        };

        let encoded = bincode::serialize(&checkpoint)?;
        std::fs::write(path, encoded)?;
        Ok(())
    }

    pub fn load_from_disk<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let data = std::fs::read(path)?;
        let checkpoint: CheckpointData = bincode::deserialize(&data)?;

        let hot_map = HotMap::new();
        hot_map.restore(checkpoint.hot_entries);

        let cold_map = ColdMap::new();
        cold_map.restore(checkpoint.file_chunks, checkpoint.chunk_index);

        Ok(Self {
            hot_map: Arc::new(hot_map),
            cold_map: Arc::new(cold_map),
        })
    }
}
