with open("src/main.rs", "r") as f:
    content = f.read()

old_read = """    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let hot_entry = self.hot_map.get(ino);"""

new_read = """    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        println!("[HLD-FS Debug] read() called: ino={}, offset={}, size={}", ino, offset, size);
        let hot_entry = self.hot_map.get(ino);
        println!("[HLD-FS Debug] hot_map.get({}) result present: {}", ino, hot_entry.is_some());"""

old_write = """    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyWrite,
    ) {
        match self.log_tier.append(data) {"""

new_write = """    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyWrite,
    ) {
        println!("[HLD-FS Debug] write() called: ino={}, offset={}, data_len={}", ino, offset, data.len());
        match self.log_tier.append(data) {"""

if old_read in content and old_write in content:
    content = content.replace(old_read, new_read).replace(old_write, new_write)
    with open("src/main.rs", "w") as f:
        f.write(content)
    print("SUCCESS: Added debug statements to src/main.rs!")
else:
    print("ERROR: Could not match the exact read/write blocks.")
