use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use parking_lot::Mutex;

pub struct DiskManager {
    file: Mutex<File>,
    capacity: AtomicU64,
}

impl DiskManager {
    pub fn new<P: AsRef<Path>>(path: P, capacity: u64) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        // Acquire an exclusive advisory lock at the kernel level.
        // LOCK_EX = Exclusive lock, LOCK_NB = Non-blocking (fail immediately if already locked)
        let lock_result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if lock_result != 0 {
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                "Failed to acquire exclusive lock on backing disk image. Is HLD-FS already mounted using this image?",
            ));
        }

        file.set_len(capacity)?;

        Ok(Self {
            file: Mutex::new(file),
            capacity: AtomicU64::new(capacity),
        })
    }

    pub fn resize(&self, new_capacity: u64) -> io::Result<()> {
        let file = self.file.lock();
        file.set_len(new_capacity)?;
        self.capacity.store(new_capacity, Ordering::SeqCst);
        Ok(())
    }

    pub fn capacity(&self) -> u64 {
        self.capacity.load(Ordering::Relaxed)
    }

    pub fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        let cap = self.capacity.load(Ordering::Relaxed);
        if offset + buf.len() as u64 > cap {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Read request exceeds disk capacity",
            ));
        }

        let mut file = self.file.lock();
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(buf)?;
        Ok(())
    }

    pub fn write_at(&self, offset: u64, buf: &[u8]) -> io::Result<()> {
        let cap = self.capacity.load(Ordering::Relaxed);
        if offset + buf.len() as u64 > cap {
            return Err(io::Error::new(
                io::ErrorKind::StorageFull,
                "Write request exceeds disk capacity",
            ));
        }

        let mut file = self.file.lock();
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(buf)?;
        file.flush()?;
        Ok(())
    }

    pub fn clone_arc(this: &Arc<Self>) -> Arc<Self> {
        Arc::clone(this)
    }
}
