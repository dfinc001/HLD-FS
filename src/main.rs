use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use fuser::{
    Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request,
    FileType, MountOption,
};
use libc::ENOENT;
use parking_lot::RwLock;

// Module declarations matching your directory layout
mod disk;
mod error;
mod log_tier;
mod metadata;
mod storage_pool;

use disk::DiskManager;
use error::{HldFsError, Result};
use log_tier::LogTier;
use metadata::{HotMap, ColdMap};
use storage_pool::StoragePool;

/// The main HldFs FUSE filesystem structure
pub struct HldFs {
    disk: Arc<DiskManager>,
    storage_pool: Arc<StoragePool>,
    hot_map: Arc<HotMap>,
    cold_map: Arc<ColdMap>,
    log_tier: Arc<LogTier>,
    // Simple in-memory directory table for FUSE inodes (Root = 1, file = 2, etc.)
    inodes: Arc<RwLock<HashMap<u64, FileMetadata>>>,
    next_inode: AtomicU64,
}

#[derive(Clone)]
struct FileMetadata {
    name: String,
    size: u64,
    kind: FileType,
    perm: u16,
}

impl HldFs {
    pub fn new(disk: Arc<DiskManager>, storage_pool: Arc<StoragePool>, log_tier: Arc<LogTier>) -> Self {
        let inodes = Arc::new(RwLock::new(HashMap::new()));
        
        // Initialize root directory (Inode 1)
        inodes.write().insert(1, FileMetadata {
            name: ".".to_string(),
            size: 0,
            kind: FileType::Directory,
            perm: 0o755,
        });

        Self {
            disk,
            storage_pool,
            hot_map: Arc::new(HotMap::new()),
            cold_map: Arc::new(ColdMap::new()),
            log_tier,
            inodes,
            next_inode: AtomicU64::new(2), // Start user files at inode 2
        }
    }
}

impl Filesystem for HldFs {
    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let inodes = self.inodes.read();
        if parent == 1 {
            let name_str = name.to_string_lossy();
            for (&ino, meta) in inodes.iter() {
                if meta.name == name_str {
                    let attr = fuser::FileAttr {
                        ino,
                        size: meta.size,
                        blocks: (meta.size + 511) / 512,
                        atime: UNIX_EPOCH,
                        mtime: UNIX_EPOCH,
                        ctime: UNIX_EPOCH,
                        crtime: UNIX_EPOCH,
                        kind: meta.kind,
                        perm: meta.perm,
                        nlink: 1,
                        uid: 501,
                        gid: 20,
                        rdev: 0,
                        flags: 0,
                        blksize: 512,
                    };
                    reply.entry(&Duration::from_secs(1), &attr, 0);
                    return;
                }
            }
        }
        reply.error(ENOENT);
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        let inodes = self.inodes.read();
        if let Some(meta) = inodes.get(&ino) {
            let attr = fuser::FileAttr {
                ino,
                size: meta.size,
                blocks: (meta.size + 511) / 512,
                atime: UNIX_EPOCH,
                mtime: UNIX_EPOCH,
                ctime: UNIX_EPOCH,
                crtime: UNIX_EPOCH,
                kind: meta.kind,
                perm: meta.perm,
                nlink: 1,
                uid: 501,
                gid: 20,
                rdev: 0,
                flags: 0,
                blksize: 512,
            };
            reply.attr(&Duration::from_secs(1), &attr);
        } else {
            reply.error(ENOENT);
        }
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let hot_entry = self.hot_map.get(ino);
        if let Some(entry) = hot_entry {
            let mut buf = vec![0u8; size as usize];
            let read_offset = entry.log_offset + offset as u64;
            if self.log_tier.read_log(read_offset, &mut buf).is_ok() {
                reply.data(&buf);
                return;
            }
        }
        reply.error(ENOENT);
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyWrite,
    ) {
        match self.log_tier.append(data) {
            Ok(log_offset) => {
                self.hot_map.insert(ino, log_offset, data.len() as u64);
                self.storage_pool.add_used(data.len() as u64);
                
                let mut inodes = self.inodes.write();
                if let Some(meta) = inodes.get_mut(&ino) {
                    meta.size = (offset + data.len() as i64) as u64;
                }

                reply.written(data.len() as u32);
            }
            Err(_) => reply.error(libc::ENOSPC),
        }
    }

    fn create(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        _flags: i32,
        reply: fuser::ReplyCreate,
    ) {
        if parent == 1 {
            let ino = self.next_inode.fetch_add(1, Ordering::SeqCst);
            let name_str = name.to_string_lossy().into_owned();

            {
                let mut inodes = self.inodes.write();
                inodes.insert(ino, FileMetadata {
                    name: name_str,
                    size: 0,
                    kind: FileType::RegularFile,
                    perm: mode as u16,
                });
            }

            let attr = fuser::FileAttr {
                ino,
                size: 0,
                blocks: 0,
                atime: UNIX_EPOCH,
                mtime: UNIX_EPOCH,
                ctime: UNIX_EPOCH,
                crtime: UNIX_EPOCH,
                kind: FileType::RegularFile,
                perm: mode as u16,
                nlink: 1,
                uid: 501,
                gid: 20,
                rdev: 0,
                flags: 0,
                blksize: 512,
            };

            reply.created(&Duration::from_secs(1), &attr, 0, 0, 0);
        } else {
            reply.error(libc::EPERM);
        }
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        let entries = vec![
            (1, FileType::Directory, "."),
            (1, FileType::Directory, ".."),
        ];

        let inodes = self.inodes.read();
        let all_entries = entries;
        let dynamic_entries: Vec<(u64, FileType, String)> = inodes
            .iter()
            .filter(|(&k, _)| k != 1)
            .map(|(&k, v)| (k, v.kind, v.name.clone()))
            .collect();

        // Combine entries for readdir iteration
        let mut index = 0;
        for (i, entry) in all_entries.into_iter().enumerate() {
            if i as i64 >= offset {
                if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                    break;
                }
            }
            index = i + 1;
        }

        for (i, entry) in dynamic_entries.into_iter().enumerate() {
            let actual_idx = index + i;
            if actual_idx as i64 >= offset {
                if reply.add(entry.0, (actual_idx + 1) as i64, entry.1, entry.2) {
                    break;
                }
            }
        }

        reply.ok();
    }
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <MOUNT_POINT>", args[0]);
        std::process::exit(1);
    }
    let mount_point = &args[1];

    // 1. Initialize Virtual Disk Image (1 GB backing file)
    let disk_path = "/tmp/hldfs_disk.img";
    let capacity = 1024 * 1024 * 1024; // 1 GB
    let disk = Arc::new(DiskManager::new(disk_path, capacity)?);
    println!("[HLD-FS] Virtual disk initialized at {} (1 GB)", disk_path);

    // 2. Initialize Storage Pool & Log Tier
    let storage_pool = Arc::new(StoragePool::new(capacity));
    let log_tier = Arc::new(LogTier::new(DiskManager::clone_arc(&disk), Arc::clone(&storage_pool), 0, capacity / 2));

    // 3. Setup Filesystem
    let fs = HldFs::new(disk, storage_pool, log_tier);

    // 4. Configure FUSE mount options
    let options = vec![
        MountOption::RW,
        MountOption::FSName("hldfs".to_string()),
    ];

    println!("[HLD-FS] Mounting file system at: {}", mount_point);
    fuser::mount2(fs, mount_point, &options)?;

    Ok(())
}

impl DiskManager {
    pub fn clone_arc(this: &Arc<Self>) -> Arc<Self> {
        Arc::clone(this)
    }
}
