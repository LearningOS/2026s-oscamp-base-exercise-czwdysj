//! # Mutex Shared State
//!
//! In this exercise, you will use `Arc<Mutex<T>>` to safely share and modify data between multiple threads.
//!
//! ## Concepts
//! - `Mutex<T>` mutex protects shared data
//! - `Arc<T>` atomic reference counting enables cross-thread sharing
//! - `lock()` acquires the lock and accesses data

use std::sync::{Arc, Mutex};
use std::thread;

pub fn concurrent_counter(n_threads: usize, count_per_thread: usize) -> usize {
    // 1. 初始化共享的计数器
    let counter = Arc::new(Mutex::new(0));

    // ⭐ 新增：创建一个动态数组，用来收集所有子线程的“句柄 (Handle)”
    let mut handles = vec![];

    // 2. 派生 n_threads 个线程
    for _ in 0..n_threads {
        let counter = Arc::clone(&counter);

        // ⭐ 修改：把派生出来的线程赋值给 handle 变量
        let handle = thread::spawn(move || {
            let mut counter = counter.lock().unwrap();

            // 每个线程独立执行 count_per_thread 次累加
            for _ in 0..count_per_thread {
                *counter += 1;
            }
        });

        // ⭐ 新增：把这个线程的句柄存进数组里
        handles.push(handle);
    }

    // ⭐ 新增核心逻辑：主线程遍历数组，等待每一个子线程干完活
    // 如果没有这一步，主线程会直接跑到底，返回一个残缺的数字 (通常是 0)
    for handle in handles {
        handle.join().unwrap();
    }

    // 3. 所有子线程都结束了，主线程最后一次拿锁，取出最终结果
    let counter = counter.lock().unwrap();
    *counter
}

/// Add elements to a shared vector concurrently using multiple threads.
/// Each thread pushes its own id (0..n_threads) to the vector.
/// Returns the sorted vector.
///
/// Hint: Use `Arc<Mutex<Vec<usize>>>`.
pub fn concurrent_collect(n_threads: usize) -> Vec<usize> {
    // 1. 初始化共享的向量
    let shared_vec = Arc::new(Mutex::new(Vec::new()));

    // ⭐ 新增：创建一个动态数组，用来收集所有子线程的“句柄 (Handle)”
    let mut handles = vec![];
    for i in 0..n_threads {
        let shared_vec = Arc::clone(&shared_vec);
        let handle = thread::spawn(move || {
            let mut vec = shared_vec.lock().unwrap();
            vec.push(i);
        });
        handles.push(handle);
    }

    // 等待所有子线程结束
    for handle in handles {
        handle.join().unwrap();
    }
    // 取出最终结果
    let mut vec = shared_vec.lock().unwrap();
    vec.sort();
    vec.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_single_thread() {
        assert_eq!(concurrent_counter(1, 100), 100);
    }

    #[test]
    fn test_counter_multi_thread() {
        assert_eq!(concurrent_counter(10, 100), 1000);
    }

    #[test]
    fn test_counter_zero() {
        assert_eq!(concurrent_counter(5, 0), 0);
    }

    #[test]
    fn test_collect() {
        let result = concurrent_collect(5);
        assert_eq!(result, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_collect_single() {
        assert_eq!(concurrent_collect(1), vec![0]);
    }
}
