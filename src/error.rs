use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HldFsError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Storage safety valve tripped: disk utilization has reached or exceeded 75%")]
    StorageFull,

    #[error("Inode not found: {0}")]
    InodeNotFound(u64),

    #[error("Chunk not found: {0}")]
    ChunkNotFound(String),

    #[error("Invalid block or chunk size: expected 128KB, got {0} bytes")]
    InvalidChunkSize(usize),

    #[error("Metadata corruption detected: {0}")]
    MetadataCorruption(String),
}

pub type Result<T> = std::result::Result<T, HldFsError>;

impl From<HldFsError> for io::Error {
    fn from(err: HldFsError) -> io::Error {
        match err {
            HldFsError::Io(e) => e,
            HldFsError::StorageFull => io::Error::new(io::ErrorKind::StorageFull, "Storage safety valve active (>= 75%)"),
            HldFsError::InodeNotFound(_) => io::Error::new(io::ErrorKind::NotFound, "Inode not found"),
            HldFsError::ChunkNotFound(_) => io::Error::new(io::ErrorKind::NotFound, "Chunk not found"),
            HldFsError::InvalidChunkSize(_) => io::Error::new(io::ErrorKind::InvalidData, "Invalid chunk size"),
            HldFsError::MetadataCorruption(msg) => io::Error::new(io::ErrorKind::InvalidData, msg),
        }
    }
}

// Optional: Helper mapping to FUSE/libc error codes if you use `fuser`
impl HldFsError {
    pub fn to_libc_error(&self) -> i32 {
        match self {
            HldFsError::Io(e) => e.raw_os_error().unwrap_or(libc::EIO),
            HldFsError::StorageFull => libc::ENOSPC,
            HldFsError::InodeNotFound(_) => libc::ENOENT,
            HldFsError::ChunkNotFound(_) => libc::ENOENT,
            HldFsError::InvalidChunkSize(_) => libc::EINVAL,
            HldFsError::MetadataCorruption(_) => libc::EIO,
        }
    }
}
