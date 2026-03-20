//! # RAII Spin Lock Guard
//! # RAII 自旋锁守卫
//!
//! In this exercise, you will implement an RAII guard for a spin lock, causing the lock to be automatically released when leaving scope.
//! This is a classic application of Rust's ownership system in systems programming.
//! 在本练习中，你将为自旋锁实现一个 RAII 守卫（Guard），使得锁在离开作用域时自动释放。
//! 这是 Rust 所有权系统在系统编程中的经典应用。
//!
//! ## Key Points
//! ## 核心要点
//! - RAII (Resource Acquisition Is Initialization) pattern / RAII 模式：资源获取即初始化
//! - `Deref` / `DerefMut` traits for transparent access / `Deref` / `DerefMut` 特性实现透明访问
//! - `Drop` trait for automatic release / `Drop` 特性实现自动释放
//! - Why manual lock/unlock is unsafe (forgetting unlock, panic without release) / 为什么手动 lock/unlock 是不安全的（忘记 unlock、发生 panic 时未释放）

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};

/// Basic SpinLock structure
/// 基础自旋锁结构体
pub struct SpinLock<T> {
    /// Atomic flag indicating if the lock is held
    /// 原子标志位，指示锁是否被持有
    locked: AtomicBool,
    /// Inner data protected by the lock, wrapped in UnsafeCell for interior mutability
    /// 由锁保护的内部数据，封装在 UnsafeCell 中以实现内部可变性
    data: UnsafeCell<T>,
}

/// Safety: T must be Send to safely transfer ownership between threads
/// 安全性：T 必须满足 Send 约束，才能在线程间安全地转移所有权
unsafe impl<T: Send> Sync for SpinLock<T> {}
unsafe impl<T: Send> Send for SpinLock<T> {}

/// Spin lock guard: RAII handle holding the lock.
/// Automatically releases the lock when SpinGuard is dropped.
/// 自旋锁守卫：持有锁的 RAII 句柄。
/// 当 SpinGuard 被丢弃（Drop）时，会自动释放锁。
pub struct SpinGuard<'a, T> {
    /// Reference to the parent SpinLock
    /// 指向父级 SpinLock 的引用
    lock: &'a SpinLock<T>,
}

impl<T> SpinLock<T> {
    /// Create a new SpinLock
    /// 创建一个新的自旋锁
    pub fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    // ------------------------------------------------------------------------
    // 实验 1: 获取锁并返回守卫 (lock)
    // 目标: 实现自旋等待逻辑，并在成功后返回一个 SpinGuard。
    // ------------------------------------------------------------------------

    /// Acquire lock, returning SpinGuard.
    /// 获取锁，并返回 SpinGuard。
    ///
    /// 1. Spin-wait to acquire lock (compare_exchange)
    /// 2. Return SpinGuard { lock: self } on success
    /// 1. 自旋等待获取锁（使用 compare_exchange）
    /// 2. 成功后返回 SpinGuard { lock: self }
    pub fn lock(&self) -> SpinGuard<'_, T> {
        // Spin until we successfully change 'locked' from false to true
        // 自旋直到我们将 'locked' 从 false 成功改为 true
        while self.locked.compare_exchange(
            false, 
            true, 
            Ordering::Acquire, 
            Ordering::Relaxed
        ).is_err() {
            // Hint to the CPU that we are in a spin loop
            // 提示 CPU 我们正处于自旋循环中
            core::hint::spin_loop();
        }
        
        // Return the RAII guard
        // 返回 RAII 守卫
        SpinGuard { lock: self }
    }
}

// ------------------------------------------------------------------------
// 实验 2: 实现 Deref 特性
// 目标: 允许通过守卫透明地访问内部数据的不可变引用。
// ------------------------------------------------------------------------

/// Implement Deref trait for SpinGuard
/// 为 SpinGuard 实现 Deref 特性
/// Return &T, obtained via self.lock.data.get()
/// 返回 &T，通过 self.lock.data.get() 获取
impl<T> Deref for SpinGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // Safety: We hold the lock, so it's safe to provide a reference to the data
        // 安全性：我们持有锁，因此可以安全地提供对数据的引用
        unsafe { &*self.lock.data.get() }
    }
}

// ------------------------------------------------------------------------
// 实验 3: 实现 DerefMut 特性
// 目标: 允许通过守卫透明地访问内部数据的可变引用。
// ------------------------------------------------------------------------

/// Implement DerefMut trait for SpinGuard
/// 为 SpinGuard 实现 DerefMut 特性
/// Return &mut T
/// 返回 &mut T
impl<T> DerefMut for SpinGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // Safety: We hold the lock exclusively, so it's safe to provide a mutable reference
        // 安全性：我们独占地持有锁，因此可以安全地提供可变引用
        unsafe { &mut *self.lock.data.get() }
    }
}

// ------------------------------------------------------------------------
// 实验 4: 实现 Drop 特性
// 目标: 在守卫离开作用域时自动释放锁。
// ------------------------------------------------------------------------

/// Implement Drop trait for SpinGuard
/// 为 SpinGuard 实现 Drop 特性
/// Set lock.locked to false (Release ordering)
/// 将 lock.locked 设置为 false (使用 Release 顺序)
impl<T> Drop for SpinGuard<'_, T> {
    fn drop(&mut self) {
        // Release the lock by setting the atomic flag to false
        // 通过将原子标志位设置为 false 来释放锁
        // Use Release ordering to ensure all preceding memory operations are visible
        // 使用 Release 顺序确保之前的所有内存操作对其他线程可见
        self.lock.locked.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_guard_auto_release() {
        let lock = SpinLock::new(0u32);
        {
            let mut guard = lock.lock();
            *guard = 42;
            // guard drops here, automatically releasing lock
        }
        // Should be able to acquire lock again
        let guard = lock.lock();
        assert_eq!(*guard, 42);
    }

    #[test]
    fn test_guard_deref() {
        let lock = SpinLock::new(String::from("hello"));
        let guard = lock.lock();
        assert_eq!(guard.len(), 5);
        assert_eq!(&*guard, "hello");
    }

    #[test]
    fn test_guard_deref_mut() {
        let lock = SpinLock::new(Vec::<i32>::new());
        {
            let mut guard = lock.lock();
            guard.push(1);
            guard.push(2);
            guard.push(3);
        }
        let guard = lock.lock();
        assert_eq!(&*guard, &[1, 2, 3]);
    }

    #[test]
    fn test_concurrent_with_guard() {
        let lock = Arc::new(SpinLock::new(0u64));
        let mut handles = vec![];

        for _ in 0..10 {
            let l = Arc::clone(&lock);
            handles.push(thread::spawn(move || {
                for _ in 0..1000 {
                    let mut guard = l.lock();
                    *guard += 1;
                    // guard automatically released
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(*lock.lock(), 10000);
    }

    #[test]
    fn test_panic_safety() {
        let lock = Arc::new(SpinLock::new(0u32));
        let l = Arc::clone(&lock);

        let result = thread::spawn(move || {
            let mut guard = l.lock();
            *guard = 42;
            panic!("intentional panic");
        })
        .join();

        assert!(result.is_err());
        // Even if thread panics, guard's Drop should release lock
        // Note: this test may have different results due to panic unwind behavior
    }
}
