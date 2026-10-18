# HLD-FS Project TODO

## ✅ Completed Milestones
- [x] **Virtual Disk & Storage Pool**: Initialized 1GB virtual disk (`DiskManager`) and structured layout with `StoragePool` tracking used space and cold allocation bounds.
- [x] **Dual-Map Metadata Architecture**: Implemented `HotMap` for fast append-only logging and `ColdMap` for deduplicated chunk tracking.
- [x] **FUSE Filesystem Integration**: Implemented essential FUSE operations (`read`, `write`, `readdir`, file creation, attributes).
- [x] **Robust Read Bounds Checking**: Clamped read requests to actual file extents in `src/main.rs`, resolving `Input/output error` (`EIO`) past EOF and enabling clean file copies (`cp --reflink=never`).
- [x] **Asynchronous Background Dedup Engine**: Powered `DedupWorker` on a Tokio runtime loop to background-sweep hot files, fingerprint 128KB blocks using BLAKE3, and migrate duplicates to the deduplicated cold tier.

## 📌 Remaining / Future Work
- [ ] **Metadata Persistence & Checkpointing**: Serialize and persist `HotMap` and `ColdMap` states to disk for durability across reboots.
- [ ] **Garbage Collection & Reference Counting**: Implement reference decrementing and space reclamation when files referencing shared cold chunks are deleted.
- [ ] **Storage Pool Monitoring**: Expose real-time storage metrics (capacity, used bytes, space saved via deduplication) via administrative endpoints or logs.
