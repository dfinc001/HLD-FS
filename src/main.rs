use std::env;
use std::sync::Arc;

use fuser::MountOption;

mod disk;
mod error;
mod fs;
mod log_tier;
mod metadata;
mod storage_pool;
mod dedup_engine;

// Re-export error types to the crate root so modules can use crate::Result and crate::HldFsError
pub use error::{Result, HldFsError};

use disk::DiskManager;
use fs::HldFs;
use log_tier::LogTier;
use storage_pool::StoragePool;
use dedup_engine::DedupWorker;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <MOUNT_POINT>", args[0]);
        std::process::exit(1);
    }
    let mount_point = &args[1];

    let disk_path = "/tmp/hldfs_disk.img";
    let initial_capacity = 1024 * 1024 * 1024; // 1 GB initial
    let disk = Arc::new(DiskManager::new(disk_path, initial_capacity)?);
    println!("[HLD-FS] Virtual disk initialized at {} (Initial: 1 GB)", disk_path);

    let storage_pool = Arc::new(StoragePool::new(initial_capacity, disk.clone(), disk_path.to_string()));
    let log_tier = Arc::new(LogTier::new(DiskManager::clone_arc(&disk), Arc::clone(&storage_pool), 0, initial_capacity / 2));

    let fs = HldFs::new(disk.clone(), storage_pool.clone(), log_tier.clone());

    let rt = tokio::runtime::Runtime::new()?;
    let metadata = std::sync::Arc::new(crate::metadata::DualMapMetadata {
        hot_map: fs.hot_map.clone(),
        cold_map: fs.cold_map.clone(),
    });
    let dedup_worker = DedupWorker::new(
        storage_pool.clone(),
        metadata,
        disk.clone(),
    );
    rt.spawn(async move {
        dedup_worker.run_background_loop().await;
    });
    std::mem::forget(rt);
    println!("[HLD-FS] Background DedupWorker spawned successfully.");

    let options = vec![
        MountOption::RW,
        MountOption::FSName("hldfs".to_string()),
    ];

    println!("[HLD-FS] Mounting file system at: {}", mount_point);
    fuser::mount2(fs, mount_point, &options)?;

    Ok(())
}
