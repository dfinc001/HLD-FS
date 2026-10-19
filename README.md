# HLD-FS: Hybrid Log-Structured Deduplicating File System

HLD-FS is an experimental tiered filesystem written in Rust using FUSE. It combines sequential log-structured writes with background chunk-based deduplication.

## Key Architecture

- **Hot tier (`HotMap` and `LogTier`):** Logs new writes and appends sequentially to maximize write throughput.
- **Cold tier (`ColdMap` and `StoragePool`):** A background worker migrates aged files into 128 KB chunks, hashes them with BLAKE3, and stores unique chunks while tracking duplicates with reference counts.
- **Capacity safety valve:** Pauses background compaction at the 75% capacity threshold.

## Prerequisites

- Rust 2021, Stable or Nightly
- FUSE support:
  - Linux: `libfuse-dev` and a FUSE-enabled environment
  - macOS: macFUSE

## Build and Install

```bash
# Install Git and Rust/Cargo using your system package manager,
# or install Rust from https://rustup.rs/
git clone https://github.com/dfinc001/HLD-FS.git
cd HLD-FS
cargo build --release
sudo install -m 755 target/release/hld-fs /usr/bin/hld-fs
```

After installation, run it with:

```bash
sudo hld-fs /mnt/my_hldfs
```

## Run

Create the mount point before starting HLD-FS:

    mkdir -p /mnt/my_hldfs

By default, HLD-FS daemonizes, mounts the filesystem, and writes logs to `/var/log/hld-fs.log`:

    sudo ./target/release/hld-fs /mnt/my_hldfs

For development or troubleshooting, use `--live` to keep the process attached to the terminal and display logs there:

    sudo ./target/release/hld-fs /mnt/my_hldfs --live

The virtual disk image is stored under `$HOME/.hldfs/`, with the mount-point name in its filename.

## Test the Filesystem

In another terminal, write and read a file:

    echo "Hello HLD-FS" > /mnt/my_hldfs/test.txt
    cat /mnt/my_hldfs/test.txt

Check whether the mount is active:

    mountpoint /mnt/my_hldfs

View daemon logs:

    sudo tail -n 50 /var/log/hld-fs.log

## Unmount

Unmount the filesystem when finished:

    sudo fusermount3 -u /mnt/my_hldfs

If `fusermount3` is unavailable, try:

    sudo umount /mnt/my_hldfs
