use std::env;
use std::fs as std_fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::os::unix::fs::PermissionsExt;

use fuser::MountOption;

mod disk;
mod error;
mod fs;
mod log_tier;
mod metadata;
mod storage_pool;
mod dedup_engine;

// Re-export error types to the crate root
pub use error::{Result, HldFsError};

use disk::DiskManager;
use fs::HldFs;
use log_tier::LogTier;
use storage_pool::StoragePool;
use dedup_engine::DedupWorker;

fn get_default_disk_path(mount_point: &str) -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let mut path = PathBuf::from(home);
    path.push(".hldfs");

    // Ensure ~/.hldfs directory exists with secure permissions (owner-only rwx -> 0o700)
    if !path.exists() {
        let _ = std_fs::create_dir_all(&path);
        if let Ok(metadata) = std_fs::metadata(&path) {
            let mut perm = metadata.permissions();
            perm.set_mode(0o700);
            let _ = std_fs::set_permissions(&path, perm);
        }
    }

    let mount_name = std::path::Path::new(mount_point)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("default");
    
    path.push(format!("{}_disk.img", mount_name));

    // If the image file already exists, ensure it is set to read-only for users (0o400)
    // When DiskManager opens it, we handle permissions or keep it owner-locked.
    if path.exists() {
        if let Ok(metadata) = std_fs::metadata(&path) {
            let mut perm = metadata.permissions();
            perm.set_mode(0o400); // Read-only for owner, blocked from direct writes
            let _ = std_fs::set_permissions(&path, perm);
        }
    }

    path
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <MOUNT_POINT>", args[0]);
        std::process::exit(1);
    }
    let mount_point = &args[1];

    let disk_path_buf = get_default_disk_path(mount_point);
    let disk_path = disk_path_buf.to_string_lossy().into_owned();

    let initial_capacity = 1024 * 1024 * 1024; // 1 GB initial
    
    // Temporarily ensure file is writable by our process during initialization/resizing,
    // or open it via OpenOptions. Note: OpenOptions with write(true) on a 0o400 file 
    // requires us to adjust permissions or handle it.
    if std::path::Path::new(&disk_path).exists() {
        let metadata = std_fs::metadata(&disk_path)?;
        let mut perm = metadata.permissions();
        perm.set_mode(0o600); // Temporarily writable for initialization
        std_fs::set_permissions(&disk_path, perm)?;
    }

    let disk = Arc::new(DiskManager::new(&disk_path, initial_capacity)?);

    // Lock the file back down to read-only (0o400) so human users cannot modify it directly,
    // while our running DiskManager keeps its active file descriptor open for writes.
    let metadata = std_fs::metadata(&disk_path)?;
    let mut perm = metadata.permissions();
    perm.set_mode(0o400);
    std_fs::set_permissions(&disk_path, perm)?;

    println!("[HLD-FS] Virtual disk securely locked and initialized at {}", disk_path);

    let storage_pool = Arc::new(StoragePool::new(initial_capacity, disk.clone(), disk_path.clone()));
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
