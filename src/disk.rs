use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use parking_lot::Mutex;

pub struct DiskManager {
    file: Mutex<File>,
    capacity: u64,
}

impl DiskManager {
    /// Opens an existing virtual disk image or creates a new pre-allocated one 
    /// of the specified capacity (in bytes).
    pub fn new<P: AsRef<Path>>(path: P, capacity: u64) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        // Ensure the backing file matches the desired virtual capacity
        file.set_len(capacity)?;

        Ok(Self {
            file: Mutex::new(file),
            capacity,
        })
    }

    /// Reads exact bytes from a specified absolute byte offset on disk into the buffer.
    pub fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        if offset + buf.len() as u64 > self.capacity {
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

    /// Writes buffer data to a specified absolute byte offset on disk.
    pub fn write_at(&self, offset: u64, buf: &[u8]) -> io::Result<()> {
        if offset + buf.len() as u64 > self.capacity {
            return Err(io::Error::new(
                io::ErrorKind::StorageFull, // or WriteZero / Other depending on needs
                "Write request exceeds disk capacity",
            ));
        }

        let mut file = self.file.lock();
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(buf)?;
        file.flush()?;
        Ok(())
    }

    /// Returns the total capacity of the virtual disk in bytes.
    pub fn capacity(&self) -> u64 {
        self.capacity
    }
}
