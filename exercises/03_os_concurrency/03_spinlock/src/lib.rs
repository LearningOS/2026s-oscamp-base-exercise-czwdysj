//! # Spin Lock
//! # 自旋锁
//!
//! In this exercise, you will implement a basic spin lock.
//! Spin locks are one of the most fundamental synchronization primitives in OS kernels.
//! 在本练习中，你将实现一个基础的自旋锁。
//! 自旋锁是操作系统内核中最基础的同步原语之一。
//!
//! ## Key Concepts
//! ## 核心概念
//! - Spin locks use busy-waiting to acquire the lock / 自旋锁使用“忙等待”的方式来获取锁
//! - `AtomicBool`'s `compare_exchange` to implement lock acquisition / 使用 `AtomicBool` 的 `compare_exchange` 来实现锁的获取
//! - `core::hint::spin_loop` to reduce CPU power consumption / 使用 `core::hint::spin_loop` 来降低 CPU 功耗
//! - `UnsafeCell` provides interior mutability / `UnsafeCell` 提供内部可变性

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};

/// Basic spin lock
pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SpinLock<T> {}
unsafe impl<T: Send> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    // ------------------------------------------------------------------------
    // 实验 1: 获取锁 (lock)
    // 目标: 实现自旋等待逻辑直到成功获取锁。
    // ------------------------------------------------------------------------

    /// Acquire lock, returning a mutable reference to inner data.
    /// 获取锁，返回内部数据的可变引用。
    ///
    /// 1. In a loop, try to change locked from false to true
    /// 2. Success uses Acquire ordering, failure uses Relaxed
    /// 3. On failure call `core::hint::spin_loop()` to hint CPU
    /// 4. On success return `&mut *self.data.get()`
    ///
    /// 1. 在循环中尝试将 locked 从 false 改为 true
    /// 2. 成功时使用 Acquire 顺序，失败时使用 Relaxed 顺序
    /// 3. 失败时调用 `core::hint::spin_loop()` 提示 CPU 处于忙等待
    /// 4. 成功后返回 `&mut *self.data.get()`
    ///
    /// # Safety
    /// Caller must ensure `unlock` is called after using the data.
    pub fn lock(&self) -> &mut T {
        // Try to acquire the lock in a loop
        // 在循环中尝试获取锁
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Hint to the CPU that we are spinning
            // 提示 CPU 我们正处于自旋状态
            core::hint::spin_loop();
        }

        // Safety: We hold the lock exclusively
        // 安全性：我们独占地持有锁
        unsafe { &mut *self.data.get() }
    }

    // ------------------------------------------------------------------------
    // 实验 2: 释放锁 (unlock)
    // 目标: 将锁状态重置为未锁定。
    // ------------------------------------------------------------------------

    /// Release lock.
    /// 释放锁。
    ///
    /// Set locked to false (using Release ordering)
    /// 将 locked 设置为 false (使用 Release 顺序)
    pub fn unlock(&self) {
        // Store false into 'locked' with Release ordering
        // 使用 Release 顺序将 false 存入 'locked'
        self.locked.store(false, Ordering::Release);
    }

    // ------------------------------------------------------------------------
    // 实验 3: 尝试获取锁 (try_lock)
    // 目标: 尝试获取一次锁，若失败不进行自旋。
    // ------------------------------------------------------------------------

    /// Try to acquire lock without spinning.
    /// Returns Some(&mut T) on success, None if lock is busy.
    /// 尝试在不自旋的情况下获取锁。
    /// 成功返回 Some(&mut T)，若锁正忙则返回 None。
    ///
    /// Single compare_exchange attempt
    /// 进行单次 compare_exchange 尝试
    pub fn try_lock(&self) -> Option<&mut T> {
        // Single attempt to acquire the lock
        // 尝试单次获取锁
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            // Safety: Success means we hold the lock
            // 安全性：成功意味着我们持有锁
            Some(unsafe { &mut *self.data.get() })
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
    fn test_basic_lock_unlock() {
        let lock = SpinLock::new(0u32);
        {
            let data = lock.lock();
            *data = 42;
            lock.unlock();
        }
        let data = lock.lock();
        assert_eq!(*data, 42);
        lock.unlock();
    }

    #[test]
    fn test_try_lock() {
        let lock = SpinLock::new(0u32);
        assert!(lock.try_lock().is_some());
        lock.unlock();
    }

    #[test]
    fn test_concurrent_counter() {
        let lock = Arc::new(SpinLock::new(0u64));
        let mut handles = vec![];

        for _ in 0..10 {
            let l = Arc::clone(&lock);
            handles.push(thread::spawn(move || {
                for _ in 0..1000 {
                    let data = l.lock();
                    *data += 1;
                    l.unlock();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let data = lock.lock();
        assert_eq!(*data, 10000);
        lock.unlock();
    }

    #[test]
    fn test_lock_protects_data() {
        let lock = Arc::new(SpinLock::new(Vec::new()));
        let mut handles = vec![];

        for i in 0..5 {
            let l = Arc::clone(&lock);
            handles.push(thread::spawn(move || {
                let data = l.lock();
                data.push(i);
                l.unlock();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let data = lock.lock();
        let mut sorted = data.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3, 4]);
        lock.unlock();
    }
}
