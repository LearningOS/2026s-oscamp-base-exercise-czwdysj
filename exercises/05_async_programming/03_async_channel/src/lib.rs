//! # Async Channel
//!
//! In this exercise, you will use `tokio::sync::mpsc` async channels to implement producer-consumer pattern.
//!
//! ## Concepts
//! - `tokio::sync::mpsc::channel` creates bounded async channels
//! - Async `send` and `recv`
//! - Channel closing mechanism (receiver returns None after all senders are dropped)

use tokio::sync::mpsc;

/// Async producer-consumer:
/// - Create a producer task that sends each element from items sequentially
/// - Create a consumer task that receives all elements and collects them into Vec for return
///
/// Hint: Set channel capacity to items.len().max(1)
pub async fn producer_consumer(items: Vec<String>) -> Vec<String> {
    // 步骤 1: 创建一个带缓冲的异步多生产者单消费者通道 (mpsc)
    let capacity = items.len().max(1);
    let (tx, mut rx) = mpsc::channel(capacity);

    // 步骤 2: 启动生产者任务
    // 使用 tokio::spawn 将生产者逻辑移至后台异步执行
    tokio::spawn(async move {
        for item in items {
            // 异步发送数据到通道
            if let Err(_) = tx.send(item).await {
                // 如果接收端已关闭，发送会失败，这里简单忽略
                break;
            }
        }
    });

    // 步骤 3: 启动消费者任务
    // 消费者循环接收，直到通道关闭（即所有发送者都被 drop 且通道内无数据）
    let consumer_handle = tokio::spawn(async move {
        let mut results = Vec::new();
        while let Some(item) = rx.recv().await {
            results.push(item);
        }
        results
    });

    // 步骤 4: 等待消费者任务完成并返回结果
    match consumer_handle.await {
        Ok(res) => res,
        Err(e) => panic!("Consumer task panicked: {:?}", e),
    }
}

/// Fan‑in pattern: multiple producers, one consumer.
/// Create `n_producers` producers, each sending `"producer {id}: message"`.
/// Consumer collects all messages, sorts them, and returns.
pub async fn fan_in(n_producers: usize) -> Vec<String> {
    // 步骤 1: 创建一个容量足够的异步通道
    let (tx, mut rx) = mpsc::channel(n_producers.max(1));

    // 步骤 2: 启动 n 个生产者任务
    for i in 0..n_producers {
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let msg = format!("producer {}: message", i);
            let _ = tx_clone.send(msg).await;
            // tx_clone 在任务结束时会被自动 drop
        });
    }

    // 步骤 3: 【关键】释放原始的发送者句柄 (tx)
    // 只有当所有发送者（包括克隆出来的）都被 drop 时，rx.recv() 才会返回 None。
    // 如果不 drop 这里的 tx，消费者将永远在 recv() 处挂起，导致程序死锁。
    drop(tx);

    // 步骤 4: 消费者收集所有消息
    let mut results = Vec::with_capacity(n_producers);
    while let Some(msg) = rx.recv().await {
        results.push(msg);
    }

    // 步骤 5: 排序并返回结果
    results.sort();
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_producer_consumer() {
        let items = vec!["hello".into(), "async".into(), "world".into()];
        let result = producer_consumer(items.clone()).await;
        assert_eq!(result, items);
    }

    #[tokio::test]
    async fn test_producer_consumer_empty() {
        let result = producer_consumer(vec![]).await;
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_fan_in() {
        let result = fan_in(3).await;
        assert_eq!(
            result,
            vec![
                "producer 0: message",
                "producer 1: message",
                "producer 2: message",
            ]
        );
    }

    #[tokio::test]
    async fn test_fan_in_single() {
        let result = fan_in(1).await;
        assert_eq!(result, vec!["producer 0: message"]);
    }
}
