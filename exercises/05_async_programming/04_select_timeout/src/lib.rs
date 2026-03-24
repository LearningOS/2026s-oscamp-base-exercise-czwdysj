//! # Select and Timeout
//!
//! In this exercise, you will use `tokio::select!` macro to implement race selection and timeout control.
//!
//! ## Concepts
//! - `tokio::select!` waits for multiple async operations simultaneously
//! - `tokio::time::timeout` timeout control
//! - The first completed branch is executed, others are cancelled

use std::future::Future;
use tokio::time::{sleep, Duration};

/// Async operation with timeout.
/// If `future` completes within `timeout_ms` milliseconds, returns Some(result).
/// Otherwise returns None.
///
/// Hint: Use `tokio::select!` or `tokio::time::timeout`.
pub async fn with_timeout<F, T>(future: F, timeout_ms: u64) -> Option<T>
where
    F: Future<Output = T>,
{
    // 步骤 1: 使用 tokio::select! 宏同时等待两个异步操作
    // 一个是传入的 future，另一个是指定时间的 sleep
    tokio::select! {
        // 如果 future 先完成，返回 Some(result)
        res = future => Some(res),
        // 如果 sleep 先完成，说明超时了，返回 None
        _ = sleep(Duration::from_millis(timeout_ms)) => None,
    }
}

/// Race two async tasks, return the result of whichever finishes first.
///
/// Hint: Use `tokio::select!` macro.
pub async fn race<F1, F2, T>(f1: F1, f2: F2) -> T
where
    F1: Future<Output = T>,
    F2: Future<Output = T>,
{
    // 步骤 1: 使用 tokio::select! 同时监听两个 Future
    // 第一个完成的分支会被执行，另一个会被取消（Dropped）
    tokio::select! {
        res = f1 => res,
        res = f2 => res,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_timeout_success() {
        let result = with_timeout(async { 42 }, 100).await;
        assert_eq!(result, Some(42));
    }

    #[tokio::test]
    async fn test_timeout_expired() {
        let result = with_timeout(
            async {
                sleep(Duration::from_millis(200)).await;
                42
            },
            50,
        )
        .await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_race_first_wins() {
        let result = race(
            async {
                sleep(Duration::from_millis(10)).await;
                "fast"
            },
            async {
                sleep(Duration::from_millis(200)).await;
                "slow"
            },
        )
        .await;
        assert_eq!(result, "fast");
    }

    #[tokio::test]
    async fn test_race_second_wins() {
        let result = race(
            async {
                sleep(Duration::from_millis(200)).await;
                "slow"
            },
            async {
                sleep(Duration::from_millis(10)).await;
                "fast"
            },
        )
        .await;
        assert_eq!(result, "fast");
    }
}
