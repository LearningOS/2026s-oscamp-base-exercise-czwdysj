//! # Channel Communication
//!
//! In this exercise, you will use `std::sync::mpsc` channels to pass messages between threads.
//!
//! ## Concepts
//! - `mpsc::channel()` creates a multiple producer, single consumer channel
//! - `Sender::send()` sends a message
//! - `Receiver::recv()` receives a message
//! - Multiple producers can be created via `Sender::clone()`

use std::sync::mpsc;
use std::thread;

/// Create a producer thread that sends each element from items into the channel.
/// The main thread receives all messages and returns them.
pub fn simple_send_recv(items: Vec<String>) -> Vec<String> {
    // 1. 创建一个通道 (Channel)，用于在线程之间安全地传递数据。
    // tx 是发送端 (Sender)，rx 是接收端 (Receiver)。
    let (tx, rx) = mpsc::channel();

    // 2. 派生一个子线程作为“生产者 (Producer)”。
    // 使用 move 关键字，将 tx 和 items 的所有权转移到子线程中。
    thread::spawn(move || {
        // 遍历 items 数组，将每一个字符串通过通道发送出去。
        for item in items {
            // send 方法返回一个 Result，如果接收端已经关闭，发送会失败。
            // 在这个简单的练习中，我们假设发送总是成功的，所以使用 unwrap()。
            tx.send(item).unwrap();
        }
        // 当这个闭包执行完毕，tx 离开作用域并被自动释放 (Drop)。
    });

    // 3. 在主线程中作为“消费者 (Consumer)”接收消息。
    // 使用 rx.recv() 或迭代器来接收消息。
    // 当所有的发送端 (tx) 都被释放后，迭代器会自动结束循环。
    let mut results = Vec::new();
    while let Ok(msg) = rx.recv() {
        results.push(msg);
    }

    results
}

/// Create `n_producers` producer threads, each sending a message in format `"msg from {id}"`.
/// Collect all messages, sort them lexicographically, and return.
///
/// Hint: Use `tx.clone()` to create multiple senders. Note that the original tx must also be dropped.
pub fn multi_producer(n_producers: usize) -> Vec<String> {
    // 1. 创建一个“多生产者单消费者 (mpsc)”通道。
    let (tx, rx) = mpsc::channel();

    // 2. 派生多个生产者线程。
    for i in 0..n_producers {
        // 通过 tx.clone() 为每一个新线程创建一个独立的发送端句柄。
        let tx_clone = tx.clone();

        thread::spawn(move || {
            // 每个线程发送一条包含自己 ID 的消息。
            let msg = format!("msg from {}", i);
            tx_clone.send(msg).unwrap();
            // 当 tx_clone 离开闭包作用域，该线程的发送端会被释放。
        });
    }

    // ⭐ 3. 关键点：在主线程中显式释放原始的 tx。
    // 如果不这样做，rx.recv() 会一直等待，因为系统认为还有一个 tx 在活动，从而导致死锁。
    drop(tx);

    // 4. 接收所有来自子线程的消息。
    // 我们可以直接使用接收端作为迭代器，它会自动阻塞并等待消息。
    let mut messages: Vec<String> = rx.into_iter().collect();

    // 5. 按照题目要求对消息进行字母序排序。
    messages.sort();

    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_send_recv() {
        let items = vec!["hello".into(), "world".into(), "rust".into()];
        let result = simple_send_recv(items.clone());
        assert_eq!(result, items);
    }

    #[test]
    fn test_simple_empty() {
        let result = simple_send_recv(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_multi_producer() {
        let result = multi_producer(3);
        assert_eq!(
            result,
            vec![
                "msg from 0".to_string(),
                "msg from 1".to_string(),
                "msg from 2".to_string(),
            ]
        );
    }

    #[test]
    fn test_multi_producer_single() {
        let result = multi_producer(1);
        assert_eq!(result, vec!["msg from 0".to_string()]);
    }
}
