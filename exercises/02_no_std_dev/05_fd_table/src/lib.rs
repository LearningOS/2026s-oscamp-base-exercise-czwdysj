//! # File Descriptor Table
//!
//! Implement a simple file descriptor (fd) table — the core data structure
//! for managing open files in an OS kernel.
//!
//! ## Background
//!
//! In the Linux kernel, each process has an fd table that maps integer fds to kernel file objects.
//! User programs perform read/write/close via fds, and the kernel looks up the corresponding
//! file object through the fd table.
//!
//! ```text
//! fd table:
//!   0 -> Stdin
//!   1 -> Stdout
//!   2 -> Stderr
//!   3 -> File("/etc/passwd")
//!   4 -> (empty)
//!   5 -> Socket(...)
//! ```
//!
//! ## Task
//!
//! Implement the following methods on `FdTable`:
//!
//! - `new()` — create an empty fd table
//! - `alloc(file)` -> `usize` — allocate a new fd, return the fd number
//!   - Prefer reusing the smallest closed fd number
//!   - If no free slot, extend the table
//! - `get(fd)` -> `Option<Arc<dyn File>>` — get the file object for an fd
//! - `close(fd)` -> `bool` — close an fd, return whether it succeeded (false if fd doesn't exist)
//! - `count()` -> `usize` — return the number of currently allocated fds (excluding closed ones)
//!
//! ## Key Concepts
//!
//! - Trait objects: `Arc<dyn File>`
//! - `Vec<Option<T>>` as a sparse table
//! - fd number reuse strategy (find smallest free slot)
//! - `Arc` reference counting and resource release

use std::sync::Arc;

/// File abstraction trait — all "files" in the kernel (regular files, pipes, sockets) implement this
/// 这是一个“文件抽象”trait：在内核世界里，**不管是普通文件、管道还是 socket**，都可以实现这个 trait，
/// 然后通过统一的 `read` / `write` 接口来读写数据。
///
/// 在本实验中，你不需要实现具体的文件类型逻辑（测试里已经给了一个 `MockFile` 实现），
/// 只需要知道：`FdTable` 里会存放 `Arc<dyn File>`，通过它来间接操作底层对象。
pub trait File: Send + Sync {
    fn read(&self, buf: &mut [u8]) -> isize;
    fn write(&self, buf: &[u8]) -> isize;
}

/// File descriptor table
pub struct FdTable {
    // TODO: Design the internal structure
    // Hint: use Vec<Option<Arc<dyn File>>>
    //       the index is the fd number, None means the fd is closed or unallocated
    //
    // 按提示，推荐的内部结构是：
    //
    //   Vec<Option<Arc<dyn File>>>
    //
    // 含义：
    // - `Vec` 的下标就是 fd 号（0,1,2,...）
    // - 某个位置为 `Some(Arc<dyn File>)` 表示该 fd 当前“打开”，指向某个文件对象
    // - 某个位置为 `None` 表示该 fd 目前“未分配 / 已关闭”，可以被后续的 `alloc` 复用
    //
    // 你的任务是在这里添加一个字段（例如 `entries: Vec<Option<Arc<dyn File>>>`），
    // 然后在下面的 `new/alloc/get/close/count` 方法中围绕这个字段实现 fd 表逻辑。
    /// 使用稀疏表保存 fd→文件对象 的映射；下标即 fd 号。
    entries: Vec<Option<Arc<dyn File>>>,
}

impl FdTable {
    /// Create an empty fd table
    pub fn new() -> Self {
        // TODO
        // 这里需要你“构造一个空的 fd 表”：
        // - 一般可以让内部的 Vec 为空（size=0），表示目前没有任何 fd 被占用
        // - 也可以选择预留容量（例如 `Vec::with_capacity(...)`），但对测试并没有硬性要求
        //
        // 返回值是 `Self`，所以需要用你在上面定义的字段来初始化 `FdTable`。
        // 这里直接创建一个 entries 为空的表，表示目前没有任何 fd 被占用。
        Self {
            entries: Vec::new(),
        }
    }

    /// Allocate a new fd, return the fd number.
    ///
    /// Prefers reusing the smallest closed fd number; if no free slot, appends to the end.
    pub fn alloc(&mut self, file: Arc<dyn File>) -> usize {
        // TODO
        // 这个方法负责“分配一个新的 fd 号”，并把传入的 `file` 挂到该 fd 上。
        //
        // 需求细化：
        // 1. 优先**复用最小的空闲 fd**：
        //    - 遍历内部 Vec，从下标 0 开始找第一个为 `None` 的位置
        //    - 如果找到了，比如 index = i：
        //      - 把该位置设为 `Some(file)`
        //      - 返回 i 作为新 fd
        //
        // 2. 若没有找到空槽（说明所有位置都是 Some）：
        //    - 把 `Some(file)` 追加到 Vec 尾部（`push`）
        //    - 返回新元素的下标（`len - 1`）
        //
        // 注意：
        // - fd 表本身不关心 `File` 的具体类型，只关心它被包在 `Arc<dyn File>` 里
        // - 这里不需要检查 fd 0/1/2 是否预留，测试里从 0 起正常分配即可
        // 先扫描现有 entries，找第一个 None 槽位用来复用 fd。
        for (fd, slot) in self.entries.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(file);
                return fd;
            }
        }
        // 若没有空闲槽位，则在末尾追加一项，并返回其下标。
        let fd = self.entries.len();
        self.entries.push(Some(file));
        fd
    }

    /// Get the file object for an fd. Returns None if the fd doesn't exist or is closed.
    pub fn get(&self, fd: usize) -> Option<Arc<dyn File>> {
        // TODO
        // 这个方法用于“通过 fd 号查找对应的文件对象”。
        //
        // 语义要求：
        // - 如果 fd 越界（fd >= 内部 Vec 长度），返回 None
        // - 如果该位置是 None（fd 已关闭/从未分配），返回 None
        // - 如果该位置是 Some(arc)，需要返回一个**克隆的 Arc**：
        //   - 可以利用 `as_ref()` / `cloned()` / `Arc::clone()` 等方式，
        //     确保引用计数 +1，而不是把所有权移走
        //
        // 提示：
        // - `self.entries.get(fd)` 返回 `Option<&Option<Arc<dyn File>>>`
        // - 你可以先匹配到 `Some(Some(arc))` 再 clone 一份返回
        // 使用 get 防止越界，然后根据槽位内容决定返回值。
        match self.entries.get(fd) {
            Some(Some(f)) => Some(Arc::clone(f)),
            _ => None,
        }
    }

    /// Close an fd. Returns true on success, false if the fd doesn't exist or is already closed.
    pub fn close(&mut self, fd: usize) -> bool {
        // TODO
        // 这个方法模拟“关闭一个 fd”：
        //
        // 行为要求：
        // - 如果 fd 越界（fd >= Vec 长度）：返回 false
        // - 如果该位置已经是 None（说明之前就没这个 fd 或已经关闭）：返回 false
        // - 如果该位置是 Some(...)：
        //   - 把该位置设为 None（例如通过 `take()` 或直接赋值）
        //   - 返回 true
        //
        // 通过将该位置设为 None，后续 `alloc` 调用就可以复用这个 fd 号了。
        if let Some(slot) = self.entries.get_mut(fd) {
            if slot.is_some() {
                *slot = None;
                return true;
            }
        }
        false
    }

    /// Return the number of currently allocated fds (excluding closed ones)
    pub fn count(&self) -> usize {
        // TODO
        // 这个方法统计“当前还打开着的 fd 数量”。
        //
        // 实现方式：
        // - 遍历内部 Vec
        // - 统计其中为 `Some(..)` 的元素个数
        // - 返回这个计数
        //
        // 注意：已经 close 掉的 fd（对应 None）不计入内。
        self.entries.iter().filter(|slot| slot.is_some()).count()
    }
}

impl Default for FdTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// Test File implementation
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockFile {
        id: usize,
        write_log: Mutex<Vec<Vec<u8>>>,
    }

    impl MockFile {
        fn new(id: usize) -> Arc<Self> {
            Arc::new(Self {
                id,
                write_log: Mutex::new(vec![]),
            })
        }
    }

    impl File for MockFile {
        fn read(&self, buf: &mut [u8]) -> isize {
            buf[0] = self.id as u8;
            1
        }
        fn write(&self, buf: &[u8]) -> isize {
            self.write_log.lock().unwrap().push(buf.to_vec());
            buf.len() as isize
        }
    }

    #[test]
    fn test_alloc_basic() {
        let mut table = FdTable::new();
        let fd = table.alloc(MockFile::new(0));
        assert_eq!(fd, 0, "first fd should be 0");
        let fd2 = table.alloc(MockFile::new(1));
        assert_eq!(fd2, 1, "second fd should be 1");
    }

    #[test]
    fn test_get() {
        let mut table = FdTable::new();
        let file = MockFile::new(42);
        let fd = table.alloc(file);
        let got = table.get(fd);
        assert!(got.is_some(), "get should return Some");
        let mut buf = [0u8; 1];
        got.unwrap().read(&mut buf);
        assert_eq!(buf[0], 42);
    }

    #[test]
    fn test_get_invalid() {
        let table = FdTable::new();
        assert!(table.get(0).is_none());
        assert!(table.get(999).is_none());
    }

    #[test]
    fn test_close_and_reuse() {
        let mut table = FdTable::new();
        let fd0 = table.alloc(MockFile::new(0)); // fd=0
        let fd1 = table.alloc(MockFile::new(1)); // fd=1
        let fd2 = table.alloc(MockFile::new(2)); // fd=2

        assert!(table.close(fd1), "closing fd=1 should succeed");
        assert!(
            table.get(fd1).is_none(),
            "get should return None after close"
        );

        // Next allocation should reuse fd=1 (smallest free)
        let fd_new = table.alloc(MockFile::new(99));
        assert_eq!(fd_new, fd1, "should reuse the smallest closed fd");

        let _ = (fd0, fd2);
    }

    #[test]
    fn test_close_invalid() {
        let mut table = FdTable::new();
        assert!(
            !table.close(0),
            "closing non-existent fd should return false"
        );
    }

    #[test]
    fn test_count() {
        let mut table = FdTable::new();
        assert_eq!(table.count(), 0);
        let fd0 = table.alloc(MockFile::new(0));
        let fd1 = table.alloc(MockFile::new(1));
        assert_eq!(table.count(), 2);
        table.close(fd0);
        assert_eq!(table.count(), 1);
        table.close(fd1);
        assert_eq!(table.count(), 0);
    }

    #[test]
    fn test_write_through_fd() {
        let mut table = FdTable::new();
        let file = MockFile::new(0);
        let fd = table.alloc(file);
        let f = table.get(fd).unwrap();
        let n = f.write(b"hello");
        assert_eq!(n, 5);
    }
}
