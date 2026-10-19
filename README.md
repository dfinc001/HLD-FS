# HLD-FS: Hybrid Log-Structured Deduplicating File System

HLD-FS is a high-performance, experimental tiered file system written in Rust using FUSE. It combines fast, sequential log-structured appends for incoming writes with background chunk-based deduplication for long-term storage efficiency.

## Key Architecture
- Hot Tier (HotMap & LogTier): New writes and appends are logged sequentially to maximize write throughput.

- Cold Tier (ColdMap & StoragePool): Background worker migrates aged files into 128KB chunks, hashes them using BLAKE3, and stores only unique chunks while tracking duplicates via reference counts.

- Safety Valve: Enforces a strict 75% capacity threshold to automatically pause background compaction and prevent storage exhaustion.


## Prerequisites
- Rust Edition: Rust 2021 (Nightly/Stable)

- FUSE Library: libfuse-dev (Linux) or macFUSE (macOS)


## Building & Running

### 1. Build the Release Binary
\`\`\`bash
cargo build --release
\`\`\`

### 2. Run as a Background Daemon (Production / Container Mode)
\`\`\`bash
umount -l /mnt/my_hldfs 2>/dev/null || true
mkdir -p /mnt/my_hldfs
nohup ./target/release/hld-fs /mnt/my_hldfs > /root/hld-fs.log 2>&1 &
\`\`\`

### 3. Interactive Development Mode (Foreground)
If you want to view live debug logs directly in your terminal:
\`\`\`bash
cargo run -- /mnt/my_hldfs
\`\`\`

### 4. Testing the Filesystem
In another terminal or session, write and read files:
\`\`\`bash
echo "Hello HLD-FS" > /mnt/my_hldfs/test.txt && cat /mnt/my_hldfs/test.txt
\`\`\`
