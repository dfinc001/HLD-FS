// src/metadata/mod.rs
pub mod hot_map;
pub mod cold_map;

pub use hot_map::HotMap;
pub use cold_map::ColdMap;

pub struct DualMapMetadata {
    pub hot_map: HotMap,
    pub cold_map: ColdMap,
}

impl DualMapMetadata {
    pub fn new() -> Self {
        Self {
            hot_map: HotMap::new(),
            cold_map: ColdMap::new(),
        }
    }
}
