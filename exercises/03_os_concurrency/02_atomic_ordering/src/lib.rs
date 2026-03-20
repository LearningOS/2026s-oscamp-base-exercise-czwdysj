//! # Memory Ordering and Synchronization
//! # 内存顺序与同步
//!
//! In this exercise, you will use correct memory ordering to implement thread synchronization primitives.
//! 在本练习中，你将使用正确的内存顺序来实现线程同步原语。
//!
//! ## Key Concepts
//! ## 核心概念
//! - `Ordering::Relaxed`: No synchronization guarantees / 无同步保证，仅保证原子性
//! - `Ordering::Acquire`: Read operation, prevents subsequent reads/writes from being reordered before this operation / 获取语义：读操作，防止随后的读写操作被重排到此操作之前
//! - `Ordering::Release`: Write operation, prevents preceding reads/writes from being reordered after this operation / 释放语义：写操作，防止前面的读写操作被重排到此操作之后
//! - `Ordering::AcqRel`: Both Acquire and Release semantics / 同时具备获取和释放语义
//! - `Ordering::SeqCst`: Sequentially consistent (global ordering) / 顺序一致性：保证全局一致的执行顺序
//!
//! ## Release-Acquire Pairing
//! ## 释放-获取配对 (Release-Acquire Pairing)
//! When thread A writes with Release, and thread B reads the same location with Acquire,
//! thread B will see all writes that thread A performed before the Release.
//! 当线程 A 以 Release 顺序写入，且线程 B 以 Acquire 顺序读取同一个位置时，
//! 线程 B 将看到线程 A 在该 Release 操作之前执行的所有写入操作。

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

// ----------------------------------------------------------------------------
// 实验 1: FlagChannel (标志位通道)
// 目标: 使用 Release-Acquire 语义在两个线程间安全地传递数据。
// ----------------------------------------------------------------------------

/// Use Release-Acquire semantics to safely pass data between two threads.
/// 使用 Release-Acquire 语义在两个线程间安全地传递数据。
///
/// `produce` writes data first, then sets flag with Release;
/// `consume` reads flag with Acquire, ensuring it sees the data.
/// `produce` 先写入数据，然后使用 Release 设置标志位；
/// `consume` 使用 Acquire 读取标志位，确保它能看到写入的数据。
pub struct FlagChannel {
    data: AtomicU32,
    ready: AtomicBool,
}

impl FlagChannel {
    pub const fn new() -> Self {
        Self {
            data: AtomicU32::new(0),
            ready: AtomicBool::new(false),
        }
    }

    /// Producer: store data first, then set ready flag.
    /// 生产者：先存储数据，然后设置 ready 标志位。
    ///
    /// TODO: Choose correct Ordering
    /// - What Ordering should be used for writing data?
    /// - What Ordering should be used for writing ready? (ensuring data writes are visible to consumer)
    /// 任务：选择正确的 Ordering
    /// - 写入 data 应该使用什么 Ordering？(提示：Relaxed)
    /// - 写入 ready 应该使用什么 Ordering？(提示：Release，确保之前的 data 写入对消费者可见)
    pub fn produce(&self, value: u32) {
        self.data.store(value, Ordering::Relaxed);
        self.ready.store(true, Ordering::Release);
        
    }

    /// Consumer: spin-wait for ready flag, then read data.
    /// 消费者：自旋等待 ready 标志位，然后读取数据。
    ///
    /// TODO: Choose correct Ordering
    /// - What Ordering should be used for reading ready? (ensuring it sees data writes from produce)
    /// - What Ordering should be used for reading data?
    /// 任务：选择正确的 Ordering
    /// - 读取 ready 应该使用什么 Ordering？(提示：Acquire，与生产者的 Release 配对)
    /// - 读取 data 应该使用什么 Ordering？(提示：Relaxed)
    pub fn consume(&self) -> u32 {
        while !self.ready.load(Ordering::Acquire) {}
        self.data.load(Ordering::Relaxed)
    }

    /// Reset channel state
    /// 重置通道状态
    pub fn reset(&self) {
        self.ready.store(false, Ordering::Relaxed);
        self.data.store(0, Ordering::Relaxed);
    }
}

// ----------------------------------------------------------------------------
// 实验 2: OnceCell (一次性初始化单元)
// 目标: 使用 SeqCst 或适当的内存顺序保证初始化仅执行一次，且所有线程都能看到初始化后的值。
// ----------------------------------------------------------------------------

/// A simple once-initializer using SeqCst.
/// Guarantees `init` is executed only once, and all threads see the initialized value.
/// 一个使用 SeqCst 的简单一次性初始化器。
/// 保证 `init` 仅执行一次，且所有线程都能看到初始化后的值。
pub struct OnceCell {
    initialized: AtomicBool,
    value: AtomicU32,
}

impl OnceCell {
    pub const fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            value: AtomicU32::new(0),
        }
    }

    /// Attempt initialization. If not yet initialized, store value and return true.
    /// If already initialized, return false.
    /// 尝试初始化。如果尚未初始化，则存储值并返回 true。
    /// 如果已经初始化，则返回 false。
    ///
    /// Hint: use `compare_exchange` to ensure only one thread succeeds.
    /// 提示：使用 `compare_exchange` 确保只有一个线程成功。
    /// 建议内存顺序：成功使用 Ordering::SeqCst，失败使用 Ordering::SeqCst (或 AcqRel/Acquire)
    pub fn init(&self, val: u32) -> bool {
        let prev = self.initialized
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);
        if prev.is_ok() {
            self.value.store(val, Ordering::SeqCst);
        }
        prev.is_ok()
    }

    /// Get value. Returns Some if initialized, otherwise None.
    /// 获取值。如果已初始化则返回 Some，否则返回 None。
    pub fn get(&self) -> Option<u32> {
        if self.initialized.load(Ordering::SeqCst) {
            Some(self.value.load(Ordering::SeqCst))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_flag_channel() {
        let ch = Arc::new(FlagChannel::new());
        let ch2 = Arc::clone(&ch);

        let producer = thread::spawn(move || {
            ch2.produce(42);
        });

        let consumer = thread::spawn(move || ch.consume());

        producer.join().unwrap();
        let val = consumer.join().unwrap();
        assert_eq!(val, 42);
    }

    #[test]
    fn test_flag_channel_large_value() {
        let ch = Arc::new(FlagChannel::new());
        let ch2 = Arc::clone(&ch);

        let producer = thread::spawn(move || {
            ch2.produce(0xDEAD_BEEF);
        });

        let val = ch.consume();
        producer.join().unwrap();
        assert_eq!(val, 0xDEAD_BEEF);
    }

    #[test]
    fn test_once_cell_init_once() {
        let cell = OnceCell::new();
        assert!(cell.init(42));
        assert!(!cell.init(100));
        assert_eq!(cell.get(), Some(42));
    }

    #[test]
    fn test_once_cell_not_initialized() {
        let cell = OnceCell::new();
        assert_eq!(cell.get(), None);
    }

    #[test]
    fn test_once_cell_concurrent() {
        let cell = Arc::new(OnceCell::new());
        let mut handles = vec![];

        for i in 0..10 {
            let c = Arc::clone(&cell);
            handles.push(thread::spawn(move || c.init(i)));
        }

        let results: Vec<bool> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        // Exactly one thread initializes successfully
        assert_eq!(results.iter().filter(|&&r| r).count(), 1);
        assert!(cell.get().is_some());
    }
}
