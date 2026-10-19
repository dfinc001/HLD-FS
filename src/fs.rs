use std::collections::HashMap;
use std::ffi::OsStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use fuser::{
    Filesystem, FileType, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request,
};
use libc::ENOENT;
use parking_lot::RwLock;

use crate::disk::DiskManager;
use crate::log_tier::LogTier;
use crate::metadata::{HotMap, ColdMap};
use crate::storage_pool::StoragePool;

#[derive(Clone)]
pub struct FileMetadata {
    pub parent: u64,
    pub name: String,
    pub size: u64,
    pub kind: FileType,
    pub perm: u16,
}

pub struct HldFs {
    pub disk: Arc<DiskManager>,
    pub storage_pool: Arc<StoragePool>,
    pub hot_map: Arc<HotMap>,
    pub cold_map: Arc<ColdMap>,
    pub log_tier: Arc<LogTier>,
    pub inodes: Arc<RwLock<HashMap<u64, FileMetadata>>>,
    pub next_inode: AtomicU64,
}

impl HldFs {
    pub fn new(disk: Arc<DiskManager>, storage_pool: Arc<StoragePool>, log_tier: Arc<LogTier>) -> Self {
        let inodes = Arc::new(RwLock::new(HashMap::new()));

        inodes.write().insert(1, FileMetadata {
            parent: 1,
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
            next_inode: AtomicU64::new(2),
        }
    }
}

impl Filesystem for HldFs {
    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let inodes = self.inodes.read();
        let name_str = name.to_string_lossy();
        for (&ino, meta) in inodes.iter() {
            if meta.parent == parent && meta.name == name_str {
                let attr = fuser::FileAttr {
                    ino,
                    size: meta.size,
                    blocks: (meta.size + 511) / 512,
                    atime: SystemTime::now(),
                    mtime: SystemTime::now(),
                    ctime: SystemTime::now(),
                    crtime: SystemTime::now(),
                    kind: meta.kind,
                    perm: meta.perm,
                    nlink: if meta.kind == FileType::Directory { 2 } else { 1 },
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
        reply.error(ENOENT);
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        let inodes = self.inodes.read();
        if let Some(meta) = inodes.get(&ino) {
            let attr = fuser::FileAttr {
                ino,
                size: meta.size,
                blocks: (meta.size + 511) / 512,
                atime: SystemTime::now(),
                mtime: SystemTime::now(),
                ctime: SystemTime::now(),
                crtime: SystemTime::now(),
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

    fn statfs(&mut self, _req: &Request<'_>, _ino: u64, reply: fuser::ReplyStatfs) {
        let total = self.storage_pool.total_capacity();
        let free = self.storage_pool.available_space();
        let bsize = 512;
        let blocks = total / bsize;
        let bfree = free / bsize;
        reply.statfs(
            blocks,
            bfree,
            bfree,
            1024,
            1024,
            bsize as u32,
            255,
            bsize as u32,
        );
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
        println!("[HLD-FS Debug] read() called: ino={}, offset={}, size={}", ino, offset, size);
        let hot_entry = self.hot_map.get(ino);
        println!("[HLD-FS Debug] hot_map.get({}) result present: {}", ino, hot_entry.is_some());
        if let Some(entry) = hot_entry {
            if offset as u64 >= entry.length {
                reply.data(&[]);
                return;
            }
            let available = entry.length - offset as u64;
            let read_size = std::cmp::min(size as u64, available) as usize;
            let mut buf = vec![0u8; read_size];
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
        println!("[HLD-FS Debug] write() called: ino={}, offset={}, data_len={}", ino, offset, data.len());
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
                    parent: parent,
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
                atime: SystemTime::now(),
                mtime: SystemTime::now(),
                ctime: SystemTime::now(),
                crtime: SystemTime::now(),
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

    fn setattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<fuser::TimeOrNow>,
        _mtime: Option<fuser::TimeOrNow>,
        _ctime: Option<std::time::SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<std::time::SystemTime>,
        _chgtime: Option<std::time::SystemTime>,
        _bkuptime: Option<std::time::SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        let mut inodes = self.inodes.write();
        if let Some(meta) = inodes.get_mut(&ino) {
            if let Some(s) = size {
                meta.size = s;
            }
            if let Some(m) = mode {
                meta.perm = m as u16;
            }

            let attr = fuser::FileAttr {
                ino,
                size: meta.size,
                blocks: (meta.size + 511) / 512,
                atime: SystemTime::now(),
                mtime: SystemTime::now(),
                ctime: SystemTime::now(),
                crtime: SystemTime::now(),
                kind: meta.kind,
                perm: meta.perm,
                nlink: 1,
                uid: uid.unwrap_or(501),
                gid: gid.unwrap_or(20),
                rdev: 0,
                flags: 0,
                blksize: 512,
            };
            reply.attr(&Duration::from_secs(1), &attr);
        } else {
            reply.error(ENOENT);
        }
    }

    fn unlink(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        reply: fuser::ReplyEmpty,
    ) {
        if parent == 1 {
            let name_str = name.to_string_lossy();
            let mut inodes = self.inodes.write();
            let mut found_ino = None;

            for (&ino, meta) in inodes.iter() {
                if meta.name == name_str {
                    found_ino = Some(ino);
                    break;
                }
            }

            if let Some(ino) = found_ino {
                inodes.remove(&ino);

                self.hot_map.remove(ino);
                if let Some(hashes) = self.cold_map.remove_file_chunks(ino) {
                    for hash in hashes {
                        self.cold_map.deref_chunk(&hash);
                    }
                }

                reply.ok();
                return;
            }
        }
        reply.error(ENOENT);
    }

    fn mkdir(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        let name_str = name.to_string_lossy().into_owned();
        let mut inodes = self.inodes.write();
        let new_ino = inodes.keys().max().copied().unwrap_or(1) + 1;

        let meta = FileMetadata {
            parent: parent,
            name: name_str,
            size: 0,
            kind: fuser::FileType::Directory,
            perm: (mode & 0xFFFF) as u16,
        };

        inodes.insert(new_ino, meta);

        let attr = fuser::FileAttr {
            ino: new_ino,
            size: 0,
            blocks: 0,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind: fuser::FileType::Directory,
            perm: (mode & 0xFFFF) as u16,
            nlink: 2,
            uid: 501,
            gid: 20,
            rdev: 0,
            flags: 0,
            blksize: 512,
        };

        reply.entry(&Duration::from_secs(1), &attr, 0);
    }

    fn rmdir(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        reply: fuser::ReplyEmpty,
    ) {
        if parent == 1 {
            let name_str = name.to_string_lossy();
            let mut inodes = self.inodes.write();
            let mut found_ino = None;

            for (&ino, meta) in inodes.iter() {
                if meta.name == name_str && meta.kind == fuser::FileType::Directory {
                    found_ino = Some(ino);
                    break;
                }
            }

            if let Some(ino) = found_ino {
                inodes.remove(&ino);

                self.hot_map.remove(ino);
                if let Some(hashes) = self.cold_map.remove_file_chunks(ino) {
                    for hash in hashes {
                        self.cold_map.deref_chunk(&hash);
                    }
                }

                reply.ok();
                return;
            }
        }
        reply.error(ENOENT);
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let inodes = self.inodes.read();
        if ino != 1 {
            if let Some(meta) = inodes.get(&ino) {
                if meta.kind != fuser::FileType::Directory {
                    reply.error(libc::ENOTDIR);
                    return;
                }
            } else {
                reply.error(ENOENT);
                return;
            }
        }

        let entries = vec![
            (1, FileType::Directory, "."),
            (1, FileType::Directory, ".."),
        ];

        let dynamic_entries: Vec<(u64, FileType, String)> = inodes
            .iter()
            .filter(|(&_k, v)| v.parent == ino)
            .map(|(&k, v)| (k, v.kind, v.name.clone()))
            .collect();

        let mut index = 0;
        for (i, entry) in entries.into_iter().enumerate() {
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
