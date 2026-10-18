# HLD-FS Project Roadmap & Status

## 🌟 Overall Goal
Implement a high-performance **Hybrid Log-Structured Deduplicating File System (HLD-FS)** in Rust, leveraging a tiered storage architecture:
* **Hot Tier (`HotMap`)**: Fast, sequential, log-structured appends for incoming writes.
* **Cold Tier (`ColdMap`)**: Chunk-addressable, BLAKE3-hashed, deduplicated long-term storage.
* **Safety & Efficiency**: Background migration worker (`DedupWorker`) with a strict **75% capacity safety valve** to prevent resource exhaustion.

---

## ✅ Completed Milestones
1. **POSIX Backbone & FUSE Setup**: Basic directory traversal, metadata tables, and FUSE operations fully functional.
2. **Metadata Coordinators**: 
   - Implemented `HotMap` for fast hot-file tracking.
   - Implemented `ColdMap` for chunk referencing and deduplication tracking.
   - Unified both under the `DualMapMetadata` coordinator (`src/metadata/mod.rs`).
3. **Storage Pool & Safety Valve**: 
   - Created `StoragePool` to manage total capacity and physical cold allocation offsets.
   - Implemented the **75% utilization threshold** (`is_safety_valve_tripped`) to pause compaction and protect storage space.
4. **Background Deduplication Engine (`DedupWorker`)**: 
   - Built the background migration loop (`src/dedup_engine.rs`).
   - Implemented **128KB chunking** and **BLAKE3 hashing**.
   - Added logic for identifying unique vs. duplicate chunks, registering references, writing to the cold tier, and cleaning up hot entries upon migration.
5. **Compilation Check**: All modules compile cleanly with `cargo check`.

---

## 🚀 Next Steps
1. **Tokio Runtime Integration**: 
   - Spawn the `DedupWorker` background task inside `src/main.rs` upon filesystem mount.
2. **Dual-Tier Read/Write Handlers**: 
   - Update FUSE read handlers to check both the `HotMap` (reading directly from the log offset) and the `ColdMap` (reassembling 128KB chunks for cold files).
3. **End-to-End Testing**: 
   - Write integration tests to verify file creation, background migration, deduplication ratios, and safety valve behavior under high loads.
