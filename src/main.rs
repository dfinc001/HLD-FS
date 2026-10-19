use std::fs as std_fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::Parser;
use daemonize::Daemonize;
use fuser::MountOption;

mod disk;
mod error;
mod fs;
mod log_tier;
mod metadata;
mod storage_pool;
mod dedup_engine;

pub use error::{Result, HldFsError};

use dedup_engine::DedupWorker;
use disk::DiskManager;
use fs::HldFs;
use log_tier::LogTier;
use storage_pool::StoragePool;

#[derive(Parser)]
#[command(name = "hld-fs", about = "Mount the HLD-FS filesystem")]
struct Args {
    /// Directory where HLD-FS will be mounted
    mount_point: PathBuf,

    /// Run in the foreground and print logs to the terminal
    #[arg(long)]
    live: bool,
}

fn get_default_disk_path(mount_point: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let mut path = PathBuf::from(home);
    path.push(".hldfs");

    if !path.exists() {
        let _ = std_fs::create_dir_all(&path);
        if let Ok(metadata) = std_fs::metadata(&path) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o700);
            let _ = std_fs::set_permissions(&path, permissions);
        }
    }

    let mount_name = Path::new(mount_point)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("default");

    path.push(format!("{mount_name}_disk.img"));

    if path.exists() {
        if let Ok(metadata) = std_fs::metadata(&path) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o400);
            let _ = std_fs::set_permissions(&path, permissions);
        }
    }

    path
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Resolve the path before daemonizing, since the daemon changes its
    // working directory to "/".
    let mount_point = std_fs::canonicalize(&args.mount_point)?;

    if !args.live {
        let log_file = std_fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/var/log/hld-fs.log")?;
        let error_log = log_file.try_clone()?;

        Daemonize::new()
            .working_directory("/")
            .stdout(log_file)
            .stderr(error_log)
            .start()?;
    }

    run_filesystem(&mount_point)
}

fn run_filesystem(
    mount_point: &Path,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mount_point_str = mount_point.to_string_lossy();
    let disk_path_buf = get_default_disk_path(&mount_point_str);
    let disk_path = disk_path_buf.to_string_lossy().into_owned();

    let initial_capacity = 1024 * 1024 * 1024; // 1 GB initial capacity

    if Path::new(&disk_path).exists() {
        let metadata = std_fs::metadata(&disk_path)?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o600);
        std_fs::set_permissions(&disk_path, permissions)?;
    }

    let disk = Arc::new(DiskManager::new(&disk_path, initial_capacity)?);

    let metadata = std_fs::metadata(&disk_path)?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o400);
    std_fs::set_permissions(&disk_path, permissions)?;

    println!(
        "[HLD-FS] Virtual disk initialized at {}",
        disk_path
    );

    let storage_pool = Arc::new(StoragePool::new(
        initial_capacity,
        disk.clone(),
        disk_path.clone(),
    ));

    let log_tier = Arc::new(LogTier::new(
        DiskManager::clone_arc(&disk),
        Arc::clone(&storage_pool),
        0,
        initial_capacity / 2,
    ));

    let fs = HldFs::new(
        disk.clone(),
        storage_pool.clone(),
        log_tier.clone(),
    );

    // Keep the Tokio runtime alive for as long as the FUSE mount is active.
    let runtime = tokio::runtime::Runtime::new()?;

    let metadata = Arc::new(crate::metadata::DualMapMetadata {
        hot_map: fs.hot_map.clone(),
        cold_map: fs.cold_map.clone(),
    });

    let dedup_worker = DedupWorker::new(
        storage_pool,
        metadata,
        disk,
    );

    runtime.spawn(async move {
        dedup_worker.run_background_loop().await;
    });

    println!("[HLD-FS] Background DedupWorker started.");
    println!("[HLD-FS] Mounting at: {}", mount_point.display());

    let options = vec![
        MountOption::RW,
        MountOption::FSName("hldfs".to_string()),
    ];

    fuser::mount2(fs, mount_point, &options)?;

    // The runtime remains alive until mount2 returns, then shuts down.
    drop(runtime);

    Ok(())
}
