//! # Manual Future Implementation
//!
//! In this exercise, you will manually implement the `Future` trait for custom types to understand the core mechanism of asynchronous runtime.
//!
//! ## Concepts
//! - `std::future::Future` trait
//! - `Poll::Ready` and `Poll::Pending`
//! - The role of `Waker`: notifying the runtime to poll again

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Countdown Future: decrements count by 1 each time it's polled,
/// returns `"liftoff!"` when count reaches 0.
pub struct CountDown {
    pub count: u32,
}

impl CountDown {
    pub fn new(count: u32) -> Self {
        Self { count }
    }
}

// TODO: Implement Future trait for CountDown
// - Output type is &'static str
// - Each poll: if count == 0, return Poll::Ready("liftoff!")
// - Otherwise count -= 1, call cx.waker().wake_by_ref(), return Poll::Pending
//
// Hint: Use `self.get_mut()` to get `&mut Self` (since self is Pin<&mut Self>)
impl Future for CountDown {
    type Output = &'static str;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // 步骤 1: 检查计数是否已归零
        if self.count == 0 {
            // 如果归零，返回 Poll::Ready，表示 Future 已完成
            Poll::Ready("liftoff!")
        } else {
            // 步骤 2: 计数减 1
            self.count -= 1;
            // 步骤 3: 唤醒执行器 (cx.waker())，告诉它本 Future 需要再次被 poll
            // 如果不唤醒，执行器可能会认为本 Future 还在等待外部事件而停止轮询
            cx.waker().wake_by_ref();
            // 步骤 4: 返回 Poll::Pending，表示任务尚未完成
            Poll::Pending
        }
    }
}

/// Yield-only-once Future: first poll returns Pending, second returns Ready(()).
/// This is the minimal example of an asynchronous state machine.
pub struct YieldOnce {
    yielded: bool,
}

impl YieldOnce {
    pub fn new() -> Self {
        Self { yielded: false }
    }
}

// TODO: Implement Future trait for YieldOnce
// - Output type is ()
// - First poll: set yielded = true, wake waker, return Pending
// - Second poll: return Ready(())
impl Future for YieldOnce {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // 检查状态机的当前状态
        if !self.yielded {
            // 第一次 poll: 设置 yielded 为 true，表示已让出一次
            self.yielded = true;
            // 唤醒 waker，确保执行器会再次调用 poll
            cx.waker().wake_by_ref();
            // 返回 Pending，让出执行权
            Poll::Pending
        } else {
            // 第二次 poll: 直接返回 Ready(())，任务完成
            Poll::Ready(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_countdown_zero() {
        let result = CountDown::new(0).await;
        assert_eq!(result, "liftoff!");
    }

    #[tokio::test]
    async fn test_countdown_three() {
        let result = CountDown::new(3).await;
        assert_eq!(result, "liftoff!");
    }

    #[tokio::test]
    async fn test_yield_once() {
        YieldOnce::new().await;
    }

    #[tokio::test]
    async fn test_countdown_large() {
        let result = CountDown::new(100).await;
        assert_eq!(result, "liftoff!");
    }
}
