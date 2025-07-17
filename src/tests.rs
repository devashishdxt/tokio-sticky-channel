use std::{collections::HashMap, num::NonZeroUsize, sync::Arc, time::Duration};

use tokio::sync::Barrier;

use crate::{SendError, TryRecvError, sticky_channel, unbounded_sticky_channel};

#[tokio::test]
async fn test_deterministic_routing_with_large_dataset() {
    let (sender, mut receivers) =
        unbounded_sticky_channel::<u64, u64>(NonZeroUsize::new(5).unwrap());

    let test_data: Vec<u64> = (0..1000).collect();
    let mut routing_table: HashMap<u64, usize> = HashMap::new();

    for &id in &test_data {
        sender.send(id, id).unwrap();
    }

    drop(sender);

    for (receiver_idx, receiver) in receivers.iter_mut().enumerate() {
        while let Some(value) = receiver.recv().await {
            if let Some(&previous_receiver) = routing_table.get(&value) {
                assert_eq!(
                    previous_receiver, receiver_idx,
                    "Value {} routed to receiver {} but was previously routed to receiver {}",
                    value, receiver_idx, previous_receiver
                );
            } else {
                routing_table.insert(value, receiver_idx);
            }
        }
    }

    assert_eq!(
        routing_table.len(),
        1000,
        "All values should be received exactly once"
    );
}

#[tokio::test]
async fn test_single_consumer_channel() {
    let (sender, mut receivers) =
        unbounded_sticky_channel::<i32, &str>(NonZeroUsize::new(1).unwrap());

    assert_eq!(receivers.len(), 1);

    sender.send(42, "hello").unwrap();
    sender.send(100, "world").unwrap();

    let msg1 = receivers[0].recv().await.unwrap();
    let msg2 = receivers[0].recv().await.unwrap();

    assert_eq!(msg1, "hello");
    assert_eq!(msg2, "world");
}

#[tokio::test]
async fn test_multiple_consumer_channel() {
    let (sender, receivers) = unbounded_sticky_channel::<i32, i32>(NonZeroUsize::new(3).unwrap());

    assert_eq!(receivers.len(), 3);

    for i in 0..10 {
        sender.send(i, i * 10).unwrap();
    }

    drop(sender);

    let mut total_received = 0;
    for mut receiver in receivers {
        while let Some(_) = receiver.recv().await {
            total_received += 1;
        }
    }

    assert_eq!(total_received, 10);
}

#[tokio::test]
async fn test_message_routing_consistency() {
    let (sender, mut receivers) =
        unbounded_sticky_channel::<&str, i32>(NonZeroUsize::new(4).unwrap());

    let test_ids = vec!["user1", "user2", "user3", "user1", "user2", "user1"];
    let mut routing_map: HashMap<String, usize> = HashMap::new();

    for (i, id) in test_ids.iter().enumerate() {
        sender.send(&id, i as i32).unwrap();
    }

    drop(sender);

    for (receiver_idx, receiver) in receivers.iter_mut().enumerate() {
        while let Some(msg_idx) = receiver.recv().await {
            let original_id = &test_ids[msg_idx as usize];
            if let Some(&prev_receiver) = routing_map.get(&original_id.to_string()) {
                assert_eq!(
                    prev_receiver, receiver_idx,
                    "ID '{}' should always route to the same receiver",
                    original_id
                );
            } else {
                routing_map.insert(original_id.to_string(), receiver_idx);
            }
        }
    }

    assert!(routing_map.contains_key("user1"));
    assert!(routing_map.contains_key("user2"));
    assert!(routing_map.contains_key("user3"));
}

#[tokio::test]
async fn test_distribution_across_receivers() {
    let (sender, mut receivers) =
        unbounded_sticky_channel::<i32, i32>(NonZeroUsize::new(3).unwrap());

    for i in 0..100 {
        sender.send(i, i).unwrap();
    }

    drop(sender);

    let mut counts = vec![0; 3];
    for (idx, receiver) in receivers.iter_mut().enumerate() {
        while let Some(_) = receiver.recv().await {
            counts[idx] += 1;
        }
    }

    assert_eq!(counts.iter().sum::<i32>(), 100);
    assert!(
        counts.iter().all(|&count| count > 0),
        "All receivers should get at least one message with 100 messages across 3 receivers"
    );
}

#[tokio::test]
async fn test_custom_hashable_types() {
    #[derive(Hash, PartialEq, Eq)]
    struct UserId {
        id: u64,
        tenant: String,
    }

    let (sender, receivers) =
        unbounded_sticky_channel::<&UserId, String>(NonZeroUsize::new(2).unwrap());

    let user1 = UserId {
        id: 1,
        tenant: "tenant_a".to_string(),
    };
    let user2 = UserId {
        id: 1,
        tenant: "tenant_b".to_string(),
    };

    sender.send(&user1, "message1".to_string()).unwrap();
    sender.send(&user2, "message2".to_string()).unwrap();
    sender.send(&user1, "message3".to_string()).unwrap();

    drop(sender);

    let mut user1_messages = Vec::new();
    let mut user2_messages = Vec::new();

    for mut receiver in receivers {
        while let Some(msg) = receiver.recv().await {
            if msg.contains("1") || msg.contains("3") {
                user1_messages.push(msg);
            } else {
                user2_messages.push(msg);
            }
        }
    }

    assert_eq!(user1_messages.len(), 2);
    assert_eq!(user2_messages.len(), 1);
}

#[tokio::test]
async fn test_send_after_receiver_dropped() {
    let (sender, receivers) = unbounded_sticky_channel::<i32, i32>(NonZeroUsize::new(2).unwrap());

    drop(receivers);

    let result = sender.send(42, 100);
    assert!(matches!(result, Err(SendError::ChannelClosed(_))));
    if let Err(SendError::ChannelClosed(value)) = result {
        assert_eq!(value, 100);
    }
}

#[tokio::test]
async fn test_try_recv_empty_channel() {
    let (sender, mut receivers) =
        unbounded_sticky_channel::<i32, i32>(NonZeroUsize::new(1).unwrap());

    let result = receivers[0].try_recv();
    assert!(matches!(result, Err(TryRecvError::Empty)));

    drop(sender);
}

#[tokio::test]
async fn test_try_recv_disconnected_channel() {
    let (sender, mut receivers) =
        unbounded_sticky_channel::<i32, i32>(NonZeroUsize::new(1).unwrap());

    drop(sender);

    let result = receivers[0].try_recv();
    assert!(matches!(result, Err(TryRecvError::Disconnected)));
}

#[tokio::test]
async fn test_send_after_all_receivers_dropped() {
    let (sender, receivers) =
        unbounded_sticky_channel::<&str, String>(NonZeroUsize::new(3).unwrap());

    sender.send("test", "message1".to_string()).unwrap();

    drop(receivers);

    let result = sender.send(&"test".to_string(), "message2".to_string());
    assert!(matches!(result, Err(SendError::ChannelClosed(_))));
}

#[tokio::test]
async fn test_recv_method() {
    let (sender, mut receivers) =
        unbounded_sticky_channel::<i32, String>(NonZeroUsize::new(1).unwrap());

    sender.send(42, "test_message".to_string()).unwrap();
    drop(sender);

    let message = receivers[0].recv().await;
    assert_eq!(message, Some("test_message".to_string()));

    let no_message = receivers[0].recv().await;
    assert_eq!(no_message, None);
}

#[tokio::test]
async fn test_recv_many_method() {
    let (sender, mut receivers) =
        unbounded_sticky_channel::<i32, i32>(NonZeroUsize::new(1).unwrap());

    for i in 0..10 {
        sender.send(0, i).unwrap();
    }
    drop(sender);

    let mut buffer = Vec::new();
    let count = receivers[0].recv_many(&mut buffer, 5).await;
    assert_eq!(count, 5);
    assert_eq!(buffer.len(), 5);

    let count2 = receivers[0].recv_many(&mut buffer, 10).await;
    assert_eq!(count2, 5);
    assert_eq!(buffer.len(), 10);

    let count3 = receivers[0].recv_many(&mut buffer, 5).await;
    assert_eq!(count3, 0);
    assert_eq!(buffer.len(), 10);
}

#[tokio::test]
async fn test_try_recv_success() {
    let (sender, mut receivers) =
        unbounded_sticky_channel::<i32, i32>(NonZeroUsize::new(1).unwrap());

    sender.send(42, 100).unwrap();

    let result = receivers[0].try_recv();
    assert_eq!(result.unwrap(), 100);

    let empty_result = receivers[0].try_recv();
    assert!(matches!(empty_result, Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn test_receiver_close_method() {
    let (sender, mut receivers) =
        unbounded_sticky_channel::<i32, i32>(NonZeroUsize::new(1).unwrap());

    sender.send(42, 100).unwrap();
    sender.send(42, 200).unwrap();

    receivers[0].close();

    let result1 = sender.send(42, 300);
    assert!(matches!(result1, Err(SendError::ChannelClosed(_))));

    let msg1 = receivers[0].recv().await;
    assert_eq!(msg1, Some(100));

    let msg2 = receivers[0].recv().await;
    assert_eq!(msg2, Some(200));

    let msg3 = receivers[0].recv().await;
    assert_eq!(msg3, None);
}

#[tokio::test]
async fn test_multiple_senders_concurrent() {
    let (sender, receivers) = unbounded_sticky_channel::<i32, i32>(NonZeroUsize::new(3).unwrap());

    let sender1 = sender.clone();
    let sender2 = sender.clone();
    let sender3 = sender;

    let barrier = Arc::new(Barrier::new(3));

    let barrier1 = barrier.clone();
    let task1 = tokio::spawn(async move {
        barrier1.wait().await;
        for i in 0..100 {
            sender1.send(i * 3, i * 3).unwrap();
        }
    });

    let barrier2 = barrier.clone();
    let task2 = tokio::spawn(async move {
        barrier2.wait().await;
        for i in 0..100 {
            sender2.send(i * 3 + 1, i * 3 + 1).unwrap();
        }
    });

    let barrier3 = barrier.clone();
    let task3 = tokio::spawn(async move {
        barrier3.wait().await;
        for i in 0..100 {
            sender3.send(i * 3 + 2, i * 3 + 2).unwrap();
        }
    });

    let _ = tokio::join!(task1, task2, task3);

    let mut total_received = 0;
    for mut receiver in receivers {
        while let Some(_) = receiver.recv().await {
            total_received += 1;
        }
    }

    assert_eq!(total_received, 300);
}

#[tokio::test]
async fn test_concurrent_receivers() {
    let (sender, receivers) = unbounded_sticky_channel::<i32, i32>(NonZeroUsize::new(4).unwrap());

    for i in 0..1000 {
        sender.send(i, i).unwrap();
    }
    drop(sender);

    let mut tasks = Vec::new();
    for mut receiver in receivers {
        let task = tokio::spawn(async move {
            let mut count = 0;
            while let Some(_) = receiver.recv().await {
                count += 1;
            }
            count
        });
        tasks.push(task);
    }

    let results = futures::future::try_join_all(tasks).await.unwrap();
    let total: i32 = results.iter().sum();

    assert_eq!(total, 1000);
    assert!(results.iter().all(|&count| count > 0));
}

#[tokio::test]
async fn test_cancel_safety_with_select() {
    let (sender, mut receivers) =
        unbounded_sticky_channel::<i32, i32>(NonZeroUsize::new(1).unwrap());

    sender.send(42, 100).unwrap();

    let result = tokio::select! {
        msg = receivers[0].recv() => Some(msg),
        _ = tokio::time::sleep(Duration::from_millis(10)) => None,
    };

    assert_eq!(result, Some(Some(100)));

    sender.send(42, 200).unwrap();
    drop(sender);

    let msg2 = receivers[0].recv().await;
    assert_eq!(msg2, Some(200));

    let msg3 = receivers[0].recv().await;
    assert_eq!(msg3, None);
}

#[tokio::test]
async fn test_zero_sized_messages() {
    let (sender, receivers) = unbounded_sticky_channel::<i32, ()>(NonZeroUsize::new(2).unwrap());

    for i in 0..10 {
        sender.send(i, ()).unwrap();
    }
    drop(sender);

    let mut total_received = 0;
    for mut receiver in receivers {
        while let Some(_) = receiver.recv().await {
            total_received += 1;
        }
    }

    assert_eq!(total_received, 10);
}

#[tokio::test]
async fn test_large_messages() {
    let (sender, mut receivers) =
        unbounded_sticky_channel::<i32, Vec<u8>>(NonZeroUsize::new(1).unwrap());

    let large_data = vec![42u8; 10_000];
    sender.send(1, large_data.clone()).unwrap();
    drop(sender);

    let received = receivers[0].recv().await.unwrap();
    assert_eq!(received.len(), 10_000);
    assert_eq!(received, large_data);
}

#[tokio::test]
async fn test_nonzero_usize_boundary() {
    let min_consumers = NonZeroUsize::new(1).unwrap();
    let (sender1, receivers1) = unbounded_sticky_channel::<i32, i32>(min_consumers);
    assert_eq!(receivers1.len(), 1);

    let large_consumers = NonZeroUsize::new(1000).unwrap();
    let (sender2, receivers2) = unbounded_sticky_channel::<i32, i32>(large_consumers);
    assert_eq!(receivers2.len(), 1000);

    sender1.send(42, 100).unwrap();
    sender2.send(42, 200).unwrap();
}

#[tokio::test]
async fn test_string_id_distribution() {
    let (sender, mut receivers) =
        unbounded_sticky_channel::<&str, i32>(NonZeroUsize::new(10).unwrap());

    let test_strings = vec![
        "short",
        "a_much_longer_string_with_many_characters",
        "123",
        "special!@#$%^&*()",
        "",
        "unicode_🦀_test",
    ];

    let mut routing_map = HashMap::new();

    for (i, s) in test_strings.iter().enumerate() {
        sender.send(s, i as i32).unwrap();
    }
    drop(sender);

    for (receiver_idx, receiver) in receivers.iter_mut().enumerate() {
        while let Some(msg_idx) = receiver.recv().await {
            let original_string = &test_strings[msg_idx as usize];
            routing_map.insert(original_string, receiver_idx);
        }
    }

    assert_eq!(routing_map.len(), test_strings.len());

    for s in &test_strings {
        assert!(routing_map.contains_key(s));
    }
}

#[tokio::test]
async fn test_recv_many_edge_cases() {
    let (sender, mut receivers) =
        unbounded_sticky_channel::<i32, i32>(NonZeroUsize::new(1).unwrap());

    sender.send(0, 1).unwrap();
    sender.send(0, 2).unwrap();
    drop(sender);

    let mut buffer = Vec::new();

    let count_zero_limit = receivers[0].recv_many(&mut buffer, 0).await;
    assert_eq!(count_zero_limit, 0);
    assert_eq!(buffer.len(), 0);

    let count_normal = receivers[0].recv_many(&mut buffer, 5).await;
    assert_eq!(count_normal, 2);
    assert_eq!(buffer, vec![1, 2]);

    let count_after_close = receivers[0].recv_many(&mut buffer, 5).await;
    assert_eq!(count_after_close, 0);
    assert_eq!(buffer.len(), 2);
}

#[tokio::test]
async fn test_bounded_basic_functionality() {
    let (sender, mut receivers) = sticky_channel::<i32, String>(NonZeroUsize::new(2).unwrap(), 10);

    assert_eq!(receivers.len(), 2);

    sender.send(42, "hello".to_string()).await.unwrap();
    sender.send(43, "world".to_string()).await.unwrap();

    let mut messages = Vec::new();
    for receiver in &mut receivers {
        while let Ok(msg) = receiver.try_recv() {
            messages.push(msg);
        }
    }

    assert_eq!(messages.len(), 2);
    assert!(messages.contains(&"hello".to_string()));
    assert!(messages.contains(&"world".to_string()));
}

#[tokio::test]
async fn test_bounded_capacity_full() {
    let (sender, mut receivers) = sticky_channel::<i32, i32>(NonZeroUsize::new(1).unwrap(), 2);

    sender.try_send(0, 1).unwrap();
    sender.try_send(0, 2).unwrap();

    let result = sender.try_send(0, 3);
    assert!(matches!(result, Err(SendError::ChannelFull(_))));
    if let Err(SendError::ChannelFull(value)) = result {
        assert_eq!(value, 3);
    }

    let msg1 = receivers[0].recv().await.unwrap();
    assert_eq!(msg1, 1);

    sender.try_send(0, 4).unwrap();
}

#[tokio::test]
async fn test_bounded_deterministic_routing() {
    let (sender, mut receivers) = sticky_channel::<&str, i32>(NonZeroUsize::new(3).unwrap(), 5);

    let test_ids = vec!["user1", "user2", "user3", "user1", "user2"];
    let mut routing_map: HashMap<String, usize> = HashMap::new();

    for (i, id) in test_ids.iter().enumerate() {
        sender.send(id, i as i32).await.unwrap();
    }

    drop(sender);

    for (receiver_idx, receiver) in receivers.iter_mut().enumerate() {
        while let Some(msg_idx) = receiver.recv().await {
            let original_id = &test_ids[msg_idx as usize];
            if let Some(&prev_receiver) = routing_map.get(&original_id.to_string()) {
                assert_eq!(
                    prev_receiver, receiver_idx,
                    "ID '{}' should always route to the same receiver",
                    original_id
                );
            } else {
                routing_map.insert(original_id.to_string(), receiver_idx);
            }
        }
    }

    assert!(routing_map.contains_key("user1"));
    assert!(routing_map.contains_key("user2"));
    assert!(routing_map.contains_key("user3"));
}

#[tokio::test]
async fn test_bounded_send_after_receiver_dropped() {
    let (sender, receivers) = sticky_channel::<i32, i32>(NonZeroUsize::new(2).unwrap(), 5);

    drop(receivers);

    let result = sender.send(42, 100).await;
    assert!(matches!(result, Err(SendError::ChannelClosed(_))));
    if let Err(SendError::ChannelClosed(value)) = result {
        assert_eq!(value, 100);
    }
}

#[tokio::test]
async fn test_bounded_try_send_after_receiver_dropped() {
    let (sender, receivers) = sticky_channel::<i32, i32>(NonZeroUsize::new(2).unwrap(), 5);

    drop(receivers);

    let result = sender.try_send(42, 100);
    assert!(matches!(result, Err(SendError::ChannelClosed(_))));
    if let Err(SendError::ChannelClosed(value)) = result {
        assert_eq!(value, 100);
    }
}

#[tokio::test]
async fn test_bounded_recv_many() {
    let (sender, mut receivers) = sticky_channel::<i32, i32>(NonZeroUsize::new(1).unwrap(), 10);

    for i in 0..5 {
        sender.send(0, i).await.unwrap();
    }
    drop(sender);

    let mut buffer = Vec::new();
    let count = receivers[0].recv_many(&mut buffer, 3).await;
    assert_eq!(count, 3);
    assert_eq!(buffer.len(), 3);

    let count2 = receivers[0].recv_many(&mut buffer, 10).await;
    assert_eq!(count2, 2);
    assert_eq!(buffer.len(), 5);
}

#[tokio::test]
async fn test_bounded_receiver_close() {
    let (sender, mut receivers) = sticky_channel::<i32, i32>(NonZeroUsize::new(1).unwrap(), 5);

    sender.send(42, 100).await.unwrap();
    sender.send(42, 200).await.unwrap();

    receivers[0].close();

    let result = sender.send(42, 300).await;
    assert!(matches!(result, Err(SendError::ChannelClosed(_))));

    let msg1 = receivers[0].recv().await;
    assert_eq!(msg1, Some(100));

    let msg2 = receivers[0].recv().await;
    assert_eq!(msg2, Some(200));

    let msg3 = receivers[0].recv().await;
    assert_eq!(msg3, None);
}

#[tokio::test]
async fn test_bounded_concurrent_senders() {
    let (sender, mut receivers) = sticky_channel::<i32, i32>(NonZeroUsize::new(3).unwrap(), 100);

    let sender1 = sender.clone();
    let sender2 = sender.clone();
    let sender3 = sender;

    let barrier = Arc::new(Barrier::new(3));

    let barrier1 = barrier.clone();
    let task1 = tokio::spawn(async move {
        barrier1.wait().await;
        for i in 0..50 {
            sender1.send(i * 3, i * 3).await.unwrap();
        }
    });

    let barrier2 = barrier.clone();
    let task2 = tokio::spawn(async move {
        barrier2.wait().await;
        for i in 0..50 {
            sender2.send(i * 3 + 1, i * 3 + 1).await.unwrap();
        }
    });

    let barrier3 = barrier.clone();
    let task3 = tokio::spawn(async move {
        barrier3.wait().await;
        for i in 0..50 {
            sender3.send(i * 3 + 2, i * 3 + 2).await.unwrap();
        }
    });

    let receive_task = tokio::spawn(async move {
        let mut total_received = 0;
        for receiver in &mut receivers {
            while let Some(_) = receiver.recv().await {
                total_received += 1;
            }
        }
        total_received
    });

    let _ = tokio::join!(task1, task2, task3);
    let total_received = receive_task.await.unwrap();

    assert_eq!(total_received, 150);
}
