//! # Atomic Operations Basics
//! # 原子操作基础
//!
//! In this exercise, you will use atomic types to implement a lock-free thread‑safe counter.
//! 在本练习中，你将使用原子类型实现一个无锁的线程安全计数器。
//!
//! ## Key Concepts
//! ## 核心概念
//! - `std::sync::atomic::AtomicU64`
//! - `fetch_add`, `fetch_sub`, `load`, `store` operations / 原子加、减、读取、存储操作
//! - `compare_exchange` lock‑free primitive / 比较并交换 (CAS) 无锁原语
//! - `Ordering` memory ordering / 内存顺序控制

use std::sync::atomic::{AtomicU64, Ordering};

pub struct AtomicCounter {
    value: AtomicU64,
}

impl AtomicCounter {
    pub const fn new(init: u64) -> Self {
        Self {
            value: AtomicU64::new(init),
        }
    }

    /// Atomically increments by 1, returns the value **before** increment.
    /// 原子性地增加 1，返回增加**之前**的值。
    ///
    /// Hint: use `fetch_add` with `Ordering::Relaxed`
    /// 提示：使用 `fetch_add` 函数，并指定内存顺序为 `Ordering::Relaxed`。
    pub fn increment(&self) -> u64 {
        // TODO: 使用 self.value.fetch_add(1, Ordering::Relaxed)
        self.value.fetch_add(1, Ordering::Relaxed)
    }

    /// Atomically decrements by 1, returns the value **before** decrement.
    /// 原子性地减少 1，返回减少**之前**的值。
    ///
    /// Hint: use `fetch_sub` with `Ordering::Relaxed`
    /// 提示：使用 `fetch_sub` 函数，内存顺序建议为 `Ordering::Relaxed`。
    pub fn decrement(&self) -> u64 {
        // TODO: 使用 self.value.fetch_sub(1, Ordering::Relaxed)
        self.value.fetch_sub(1, Ordering::Relaxed)
    }

    /// Gets the current value.
    /// 获取当前计数器的值。
    ///
    /// Hint: use `load` with `Ordering::Relaxed`
    /// 提示：使用 `load` 函数读取当前值，内存顺序使用 `Ordering::Relaxed`。
    pub fn get(&self) -> u64 {
        // TODO: 使用 self.value.load(Ordering::Relaxed)
        self.value.load(Ordering::Relaxed)
    }

    /// Atomic CAS (Compare-And-Swap) operation.
    /// If current value equals `expected`, set to `new_val` and return Ok(expected).
    /// Otherwise return Err(actual current value).
    /// 原子 CAS（比较并交换）操作。
    /// 如果当前值等于 `expected`，则将其设置为 `new_val` 并返回 Ok(expected)。
    /// 否则返回 Err(当前实际的值)。
    ///
    /// Hint: use `compare_exchange` with success ordering `Ordering::AcqRel` and failure ordering `Ordering::Acquire`
    /// 提示：使用 `compare_exchange` 函数。成功时使用 `Ordering::AcqRel`，失败时使用 `Ordering::Acquire`。
    pub fn compare_and_swap(&self, expected: u64, new_val: u64) -> Result<u64, u64> {
        // TODO: 使用 self.value.compare_exchange(expected, new_val, Ordering::AcqRel, Ordering::Acquire)
        self.value
            .compare_exchange(expected, new_val, Ordering::AcqRel, Ordering::Acquire)
    }

    /// Multiply the value atomically using a CAS loop.
    /// Returns the value **before** multiplication.
    /// 使用 CAS 循环原子性地实现乘法操作。
    /// 返回乘法操作**之前**的值。
    ///
    /// Hint: read current value in loop, compute new value, try CAS to update, retry on failure.
    /// 提示：在循环中读取当前值，计算新值，尝试用 CAS 更新。如果失败则重试。
    pub fn fetch_multiply(&self, multiplier: u64) -> u64 {
        // TODO: 实现 CAS 循环
        loop {
            let current = self.get(); // 获取当前值
            let new = current * multiplier; // 计算乘法结果
            match self.compare_and_swap(current, new) {
                // 尝试更新
                Ok(old) => return old, // 成功则返回旧值
                Err(_) => continue,    // 失败（说明被其他线程抢先了）则继续循环
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_basic_ops() {
        let c = AtomicCounter::new(0);
        assert_eq!(c.increment(), 0);
        assert_eq!(c.increment(), 1);
        assert_eq!(c.get(), 2);
        assert_eq!(c.decrement(), 2);
        assert_eq!(c.get(), 1);
    }

    #[test]
    fn test_cas_success() {
        let c = AtomicCounter::new(10);
        assert_eq!(c.compare_and_swap(10, 20), Ok(10));
        assert_eq!(c.get(), 20);
    }

    #[test]
    fn test_cas_failure() {
        let c = AtomicCounter::new(10);
        assert_eq!(c.compare_and_swap(5, 20), Err(10));
        assert_eq!(c.get(), 10);
    }

    #[test]
    fn test_fetch_multiply() {
        let c = AtomicCounter::new(3);
        let old = c.fetch_multiply(4);
        assert_eq!(old, 3);
        assert_eq!(c.get(), 12);
    }

    #[test]
    fn test_concurrent_increment() {
        let counter = Arc::new(AtomicCounter::new(0));
        let mut handles = vec![];

        for _ in 0..10 {
            let c = Arc::clone(&counter);
            handles.push(thread::spawn(move || {
                for _ in 0..1000 {
                    c.increment();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(counter.get(), 10000);
    }
}
