# HLD-FS Project TODO

## ✅ Completed Milestones
- [x] **Virtual Disk & Storage Pool**: Initialized virtual disk (`DiskManager`) and structured layout with `StoragePool` tracking used space and cold allocation bounds.

- [x] **Dual-Map Metadata Architecture**: Implemented `HotMap` for fast append-only logging and `ColdMap` for deduplicated chunk tracking.

- [x] **FUSE Filesystem Integration**: Implemented essential FUSE operations (`read`, `write`, `readdir`, file creation, attributes).

- [x] **Robust Read Bounds Checking**: Clamped read requests to actual file extents in `src/main.rs`, resolving `Input/output error` (`EIO`) past EOF and enabling clean file copies.

- [x] **Asynchronous Background Dedup Engine**: Powered `DedupWorker` on a Tokio runtime loop to background-sweep hot files, fingerprint 128KB blocks using BLAKE3, and migrate duplicates to the deduplicated cold tier.

- [x] **Secure Persistent Storage Architecture**: Shifted backing images to a per-mount hidden user directory (`~/.hldfs/`) set to strict `0o700` permissions.

- [x] **Read-Only File Protection & Kernel Locking**: Enforced `0o400` (Owner Read-Only) permissions on backing image files while unmounted to prevent manual tampering, combined with exclusive kernel-level file locking (`flock`) to prevent accidental double-mounting and concurrency conflicts.

- [x] **Dynamic Auto-Expanding Storage Pool**: Implemented auto-growing virtual disk chunks (1GB increments) with host `statvfs` exhaustion protection.

- [x] **Metadata Persistence & Checkpointing**: Serialized and persisted `HotMap` and `ColdMap` states using `bincode` and `serde` for full durability across reboots.

- [x] **Garbage Collection & Reference Counting**: Implemented chunk reference decrementing and space reclamation when files referencing shared cold chunks are deleted.

- [x] **Storage Pool Monitoring & Metrics**: Exposed real-time storage metrics, capacity tracking, and space savings via deduplication ratio analytics (`dedup_ratio()`).
