# HLD-FS Project TODO

## ✅ Completed Milestones

- [x] **Virtual Disk & Storage Pool:** Initialized the virtual disk through `DiskManager`; `StoragePool` tracks used space and cold-allocation bounds.
- [x] **Dual-Map Metadata Architecture:** Implemented `HotMap` for fast append-only logging and `ColdMap` for deduplicated chunk tracking.
- [x] **FUSE Filesystem Integration:** Implemented essential FUSE operations, including `read`, `write`, `readdir`, file creation, and attribute handling.
- [x] **Robust Read Bounds Checking:** Clamped reads to actual file extents, resolving `EIO` errors past EOF and enabling clean file copies.
- [x] **Asynchronous Background Deduplication:** Runs `DedupWorker` on a Tokio runtime loop to sweep hot files, fingerprint 128 KB blocks with BLAKE3, and migrate duplicates to the cold tier.
- [x] **Secure Persistent Storage Location:** Stores backing images under a per-user `~/.hldfs/` directory with `0o700` permissions.
- [x] **Backing-Image Permissions & Locking:** Sets backing images to `0o400` when not being initialized and uses exclusive `flock` locking to help prevent double mounts and concurrent access.
- [x] **Dynamic Storage Expansion:** Grows the virtual disk in 1 GB increments and checks host capacity with `statvfs`.
- [x] **Metadata Persistence & Checkpointing:** Serializes and persists `HotMap` and `ColdMap` state with `bincode` and `serde`.
- [x] **Garbage Collection & Reference Counting:** Decrements chunk references and reclaims space when files referencing shared cold chunks are deleted.
- [x] **Storage Metrics:** Tracks capacity and exposes deduplication savings through `dedup_ratio()`.
- [x] **Daemon and Foreground Modes:** Starts in the background by default; `--live` keeps the process in the foreground for troubleshooting.
- [x] **Daemon Logging:** Redirects daemon stdout and stderr to `/var/log/hld-fs.log`.
- [x] **Runtime Lifetime Fix:** Keeps the Tokio runtime alive while the FUSE mount is active.
- [x] **Basic Runtime Verification:** Confirmed the daemon process runs and tested filesystem reads, writes, and background migration.

## 🚧 Next Up

- [ ] **Add a `--disk` Option:** Allow users to select a backing image instead of deriving its path from the mount-point name.
- [ ] **Add Formatting Support:** Implement `hld-fs mkfs` to create and initialize a backing image explicitly; consider exposing it later as `mkfs.hldfs`.
- [ ] **Add Systemd Service Setup:** Add an installation option that generates a service running HLD-FS with `--live`, so systemd can supervise it and restore the mount after reboot.
- [ ] **Handle Non-Root Service Installation:** If service installation is requested without root privileges, save the unit file in the user’s home directory and print instructions for installing it system-wide.
- [ ] **Document Boot-Time Mounting:** Document the systemd service approach. Do not generate an `/etc/fstab` entry until a compatible FUSE mount helper exists.
- [ ] **Improve Daemon Startup Reporting:** Ensure startup or mount failures are visible to the invoking user; daemonization can otherwise make failures less obvious.
- [ ] **Test Recovery and Persistence:** Test reboot/restart behavior, metadata recovery, unclean shutdowns, and backing-image persistence.
- [ ] **Document Container Requirements:** Document FUSE device access, required privileges/capabilities, persistent storage, and logging for container deployments.
