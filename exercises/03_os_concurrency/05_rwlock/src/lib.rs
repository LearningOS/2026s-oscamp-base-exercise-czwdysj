//! # Read-Write Lock (Writer-Priority)
//! # 读写锁 (写者优先)
//!
//! In this exercise, you will implement a **writer-priority** read-write lock from scratch using atomics.
//! Multiple readers may hold the lock concurrently; a writer holds it exclusively.
//! 在本练习中，你将使用原子操作从零实现一个**写者优先**的读写锁。
//! 多个读者可以同时持有锁；写者则独占持有锁。
//!
//! **Note:** Rust's standard library already provides [`std::sync::RwLock`]. This exercise implements for learning the protocol and policy without using the standard one.
//! a minimal version
//! **注意：** Rust 标准库已经提供了 [`std::sync::RwLock`]。本练习实现一个极简版本，
//! 用于学习同步协议和策略，而不直接使用标准库。
//!
//! ## Common policies for read-write locks
//! ## 读写锁的常见策略
//! Different implementations can give different **priority** when both readers and writers are waiting:
//! 当读者和写者都在等待时，不同的实现可以给予不同的**优先级**：
//!
//! - **Reader-priority (读者优先)**: New readers are allowed to enter while a writer is waiting, so writers
//!   may be starved if readers keep arriving.
//!   (读者优先)：当有写者等待时，仍允许新读者进入，如果读者持续到达，写者可能会“饥饿”。
//! - **Writer-priority (写者优先)**: Once a writer is waiting, no new readers are admitted until that writer
//!   has run; this exercise implements this policy.
//!   (写者优先)：一旦有写者在等待，就不再允许新读者进入，直到该写者运行完毕；本练习实现此策略。
//! - **Read-write fair (读写公平)**: Requests are served in a fair order (e.g. FIFO or round-robin), so
//!   neither readers nor writers are systematically starved.
//!   (读写公平)：按公平顺序（如 FIFO）处理请求，确保读者和写者都不会系统性地饥饿。
//!
//! ## Key Concepts
//! ## 核心概念
//! - **Readers**: share access; many threads can hold a read lock at once.
//!   (读者)：共享访问；多个线程可以同时持有读锁。
//! - **Writer**: exclusive access; only one writer, and no readers while the writer holds the lock.
//!   (写者)：独占访问；只有一个写者，且写者持有锁时不能有读者。
//! - **Writer-priority (this implementation)**: when at least one writer is waiting, new readers block
//!   until the writer runs.
//!   (写者优先)：当至少有一个写者在等待时，新读者将被阻塞，直到写者运行。
//!
//! ## State (single atomic)
//! ## 状态控制 (单个原子变量)
//! We use one `AtomicU32`: low bits = reader count, two flags = writer holding / writer waiting.
//! All logic is implemented with compare_exchange and load/store; no use of `std::sync::RwLock`.
//! 我们使用一个 `AtomicU32`：低位 = 读者计数，两个标志位 = 写者持有 / 写者等待。
//! 所有逻辑都通过 compare_exchange 和 load/store 实现；不使用 `std::sync::RwLock`。

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU32, Ordering};

/// Maximum number of concurrent readers (fits in state bits).
/// 最大并发读者数（占用状态变量的低位）。
const READER_MASK: u32 = (1 << 30) - 1;
/// Bit set when a writer holds the lock.
/// 写者持有锁时的标志位。
const WRITER_HOLDING: u32 = 1 << 30;
/// Bit set when at least one writer is waiting (writer-priority: block new readers).
/// 至少有一个写者在等待时的标志位（写者优先：阻塞新读者）。
const WRITER_WAITING: u32 = 1 << 31;

/// Writer-priority read-write lock. Implemented from scratch; does not use `std::sync::RwLock`.
/// 写者优先的读写锁。从零实现，不使用 `std::sync::RwLock`。
pub struct RwLock<T> {
    /// Atomic state: [WRITER_WAITING | WRITER_HOLDING | READER_COUNT]
    /// 原子状态位：[写者等待 | 写者持有 | 读者计数]
    state: AtomicU32,
    /// Inner data protected by the lock
    /// 由锁保护的内部数据
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for RwLock<T> {}
unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    /// Create a new RwLock
    /// 创建一个新的读写锁
    pub const fn new(data: T) -> Self {
        Self {
            state: AtomicU32::new(0),
            data: UnsafeCell::new(data),
        }
    }

    // ------------------------------------------------------------------------
    // 实验 1: 获取读锁 (read)
    // 目标: 实现写者优先的读锁获取逻辑。
    // ------------------------------------------------------------------------

    /// Acquire a read lock. Blocks (spins) until no writer holds and no writer is waiting (writer-priority).
    /// 获取读锁。阻塞（自旋）直到没有写者持有锁且没有写者在等待（写者优先）。
    ///
    /// 1. In a loop, load state (Acquire).
    /// 2. If WRITER_HOLDING or WRITER_WAITING is set, spin_loop and continue.
    /// 3. If reader count (state & READER_MASK) is already READER_MASK, spin and continue.
    /// 4. Try compare_exchange(s, s + 1, AcqRel, Acquire); on success return RwLockReadGuard.
    ///
    /// 1. 在循环中加载状态 (Acquire 顺序)。
    /// 2. 如果 WRITER_HOLDING 或 WRITER_WAITING 被设置，则自旋并继续（写者优先）。
    /// 3. 如果读者计数达到最大值，则自旋并继续。
    /// 4. 尝试 compare_exchange(s, s + 1, AcqRel, Acquire)；成功后返回 RwLockReadGuard。
    ///
    /// 建议使用函数: self.state.load, core::hint::spin_loop, self.state.compare_exchange
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        // 1. 开启一个无限循环 (自旋等待)
        loop {
            // 步骤 1: 拍一张当前的“状态快照” (使用 Acquire 保证读到的状态是最新的)
            let s = self.state.load(Ordering::Acquire);

            // 步骤 2: 检查是否有写者持有锁，或者有写者在等待 (写者优先的核心逻辑)
            // 使用按位与(&)操作：只要这两个标志位中有一个是 1，结果就不为 0
            if (s & (WRITER_HOLDING | WRITER_WAITING)) != 0 {
                // 乖乖让路，告诉 CPU 当前在空转，降低功耗
                core::hint::spin_loop();
                continue;
            }

            // 步骤 3: 检查读者数量是否已经达到物理极限 (防溢出)
            // 提取低 30 位，看它是不是已经全满了
            if (s & READER_MASK) == READER_MASK {
                core::hint::spin_loop();
                continue;
            }

            // 步骤 4: 尝试原子性地把读者数量加 1
            // 因为前面确认了高两位都是 0，所以这里直接 s + 1 就是读者数量 +1
            match self
                .state
                .compare_exchange(s, s + 1, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => {
                    // 抢占成功！跳出循环，返回一个包装好的智能指针 (Guard)
                    return RwLockReadGuard { lock: self };
                }
                Err(_) => {
                    // 抢占失败（就在你准备加 1 的瞬间，别人修改了状态）
                    // 没关系，直接进入下一轮循环重新尝试
                    continue;
                }
            }
        }
    }

    // ------------------------------------------------------------------------
    // 实验 2: 获取写锁 (write)
    // 目标: 实现写者优先的写锁获取逻辑。
    // ------------------------------------------------------------------------

    /// Acquire the write lock. Blocks until no readers and no other writer.
    /// 获取写锁。阻塞直到没有读者且没有其他写者。
    ///
    /// 1. Set WRITER_WAITING first: fetch_or(WRITER_WAITING, Release) so new readers will block.
    /// 2. In a loop: load state; if any readers (READER_MASK) or WRITER_HOLDING, spin_loop and continue.
    /// 3. Try compare_exchange to take the lock (set WRITER_HOLDING, clear WRITER_WAITING if no other writer waits).
    /// 4. On success return RwLockWriteGuard.
    ///
    /// 1. 首先设置 WRITER_WAITING：使用 fetch_or(WRITER_WAITING, Release)，使新读者阻塞。
    /// 2. 在循环中：加载状态；如果存在读者或写者持有锁，则自旋并继续。
    /// 3. 尝试 compare_exchange 获取锁（设置 WRITER_HOLDING，并在没有其他写者等待时清除 WRITER_WAITING）。
    /// 4. 成功后返回 RwLockWriteGuard。
    ///
    /// 建议使用函数: self.state.fetch_or, self.state.load, core::hint::spin_loop, self.state.compare_exchange
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        // 步骤 1: 设置写者等待标志位 (WRITER_WAITING)
        // 告诉新来的读者：“嘿，有写者在排队了，你们先别进去了！” (写者优先的核心体现)
        self.state.fetch_or(WRITER_WAITING, Ordering::Release);

        loop {
            // 步骤 2: 拍一张当前的状态快照
            let s = self.state.load(Ordering::Acquire);

            // 步骤 3: 检查是否可以“独占”
            // 只要还有读者在读 (READER_MASK)，或者有其他写者在写 (WRITER_HOLDING)，就得继续等
            if (s & READER_MASK) != 0 || (s & WRITER_HOLDING) != 0 {
                // 还在有人用，继续自旋
                core::hint::spin_loop();
                continue;
            }

            // 步骤 4: 尝试原子性地抢占写锁
            // 我们希望将状态设置为 WRITER_HOLDING，同时尝试清除 WRITER_WAITING 标志
            // (注意：在此极简实现中，由于没有等待计数器，我们默认清除等待位；如果有多个写者，下一个写者会重新设置它)
            let new_s = (s & !WRITER_WAITING) | WRITER_HOLDING;
            match self
                .state
                .compare_exchange(s, new_s, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => {
                    // 抢占成功！返回写锁守卫
                    return RwLockWriteGuard { lock: self };
                }
                Err(_) => {
                    // 状态在检查和尝试抢占之间变了，继续循环重试
                    continue;
                }
            }
        }
    }
}

// ------------------------------------------------------------------------
// 实验 3: 实现读锁守卫 (RwLockReadGuard)
// 目标: 实现 Deref 和 Drop 以管理读锁生命周期。
// ------------------------------------------------------------------------

/// Guard for a read lock; releases the read lock on drop.
/// 读锁守卫；在 Drop 时释放读锁。
pub struct RwLockReadGuard<'a, T> {
    lock: &'a RwLock<T>,
}

/// Implement Deref for RwLockReadGuard
/// 为 RwLockReadGuard 实现 Deref
/// Return shared reference to data: unsafe { &*self.lock.data.get() }
impl<T> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // 安全性：我们持有读锁，且读锁逻辑保证了此刻没有写者在修改数据
        // 因此我们可以安全地通过 UnsafeCell 获取内部数据的不可变引用
        unsafe { &*self.lock.data.get() }
    }
}

/// Implement Drop for RwLockReadGuard
/// 为 RwLockReadGuard 实现 Drop
/// Decrement reader count: self.lock.state.fetch_sub(1, Ordering::Release)
impl<T> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        // 释放读锁：将原子状态中的读者计数减 1
        // 使用 Release 语义确保之前的读取操作不会被重排到原子减少之后
        self.lock.state.fetch_sub(1, Ordering::Release);
    }
}

// ------------------------------------------------------------------------
// 实验 4: 实现写锁守卫 (RwLockWriteGuard)
// 目标: 实现 Deref, DerefMut 和 Drop 以管理写锁生命周期。
// ------------------------------------------------------------------------

/// Guard for a write lock; releases the write lock on drop.
/// 写锁守卫；在 Drop 时释放写锁。
pub struct RwLockWriteGuard<'a, T> {
    lock: &'a RwLock<T>,
}

/// Implement Deref for RwLockWriteGuard
/// 为 RwLockWriteGuard 实现 Deref
/// Return shared reference to data: unsafe { &*self.lock.data.get() }
impl<T> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // 安全性：我们持有独占的写锁，可以安全地读取数据
        unsafe { &*self.lock.data.get() }
    }
}

/// Implement DerefMut for RwLockWriteGuard
/// 为 RwLockWriteGuard 实现 DerefMut
/// Return mutable reference: unsafe { &mut *self.lock.data.get() }
impl<T> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // 安全性：我们持有独占的写锁，可以安全地获取内部数据的可变引用
        unsafe { &mut *self.lock.data.get() }
    }
}

/// Implement Drop for RwLockWriteGuard
/// 为 RwLockWriteGuard 实现 Drop
/// Clear writer bits so lock is free: self.lock.state.fetch_and(!WRITER_HOLDING, Ordering::Release)
impl<T> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        // 释放写锁：通过位与操作清除 WRITER_HOLDING 标志位
        // 使用 Release 语义确保之前的写操作对其他线程可见
        self.lock
            .state
            .fetch_and(!WRITER_HOLDING, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_multiple_readers() {
        let lock = Arc::new(RwLock::new(0u32));
        let mut handles = vec![];
        for _ in 0..10 {
            let l = Arc::clone(&lock);
            handles.push(thread::spawn(move || {
                let g = l.read();
                assert_eq!(*g, 0);
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn test_writer_excludes_readers() {
        let lock = Arc::new(RwLock::new(0u32));
        let lock_w = Arc::clone(&lock);
        let writer = thread::spawn(move || {
            let mut g = lock_w.write();
            *g = 42;
        });
        writer.join().unwrap();
        let g = lock.read();
        assert_eq!(*g, 42);
    }

    #[test]
    fn test_concurrent_reads_after_write() {
        let lock = Arc::new(RwLock::new(Vec::<i32>::new()));
        {
            let mut g = lock.write();
            g.push(1);
            g.push(2);
        }
        let mut handles = vec![];
        for _ in 0..5 {
            let l = Arc::clone(&lock);
            handles.push(thread::spawn(move || {
                let g = l.read();
                assert_eq!(g.len(), 2);
                assert_eq!(&*g, &[1, 2]);
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn test_concurrent_writes_serialized() {
        let lock = Arc::new(RwLock::new(0u64));
        let mut handles = vec![];
        for _ in 0..10 {
            let l = Arc::clone(&lock);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let mut g = l.write();
                    *g += 1;
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(*lock.read(), 1000);
    }
}
