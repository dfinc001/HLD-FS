# HLD-FS: Hybrid Log-Structured Deduplicating File System

HLD-FS is a high-performance, experimental tiered file system written in Rust using FUSE. It combines fast, sequential log-structured appends for incoming writes with background chunk-based deduplication for long-term storage efficiency.

## Key Architecture
- Hot Tier (HotMap & LogTier): New writes and appends are logged sequentially to maximize write throughput.
- Cold Tier (ColdMap & StoragePool): Background worker migrates aged files into 128KB chunks, hashes them using BLAKE3, and stores only unique chunks while tracking duplicates via reference counts.
- Safety Valve: Enforces a strict 75% capacity threshold to automatically pause background compaction and prevent storage exhaustion.

## Prerequisites
- Rust Edition: Rust 2021 (Nightly/Stable)
- FUSE Library: libfuse-dev (Linux) or macFUSE (macOS)

## Building & Testing
1. Build: cargo build --release
2. Create Mount Point: mkdir -p /tmp/hldfs_mnt
3. Run: cargo run -- /tmp/hldfs_mnt
4. Test in another terminal: echo "Hello HLD-FS" > /tmp/hldfs_mnt/test.txt && cat /tmp/hldfs_mnt/test.txt
