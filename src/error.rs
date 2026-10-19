// src/error.rs
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HldFsError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Storage pool is full")]
    StorageFull,

    #[error("Not found")]
    NotFound,

    #[error("Invalid operation")]
    InvalidOperation,
}

pub type Result<T> = std::result::Result<T, HldFsError>;
