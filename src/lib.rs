//! A sticky channel implementation for Tokio that routes messages to specific receivers based on ID hashing.
//!
//! This crate provides message passing channels where messages with the same ID are consistently
//! delivered to the same receiver. This is useful for workload distribution scenarios where you need
//! to ensure that related messages are processed by the same consumer for ordering guarantees or
//! stateful processing.
//!
//! # Key Features
//!
//! - **Deterministic routing**: Messages with the same ID always go to the same receiver
//! - **Multiple producers**: Senders can be cloned and used from multiple threads
//! - **Async and sync receiving**: Support for both `async` and non-blocking receive operations
//! - **Cancel-safe**: All operations work correctly with `tokio::select!`
//! - **Bounded and unbounded channels**: Choose between memory-bounded or unbounded channels
//!
//! # Channel Types
//!
//! ## Unbounded Sticky Channel
//!
//! ```rust
//! use tokio_sticky_channel::unbounded_sticky_channel;
//! use std::num::NonZeroUsize;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create an unbounded sticky channel with 3 consumers
//!     let (sender, mut receivers) = unbounded_sticky_channel::<String, i32>(
//!         NonZeroUsize::new(3).unwrap()
//!     );
//!
//!     // Send messages with IDs - same ID always goes to same receiver
//!     sender.send(&"user-123".to_string(), 42).unwrap();
//!     sender.send(&"user-456".to_string(), 24).unwrap();
//!     sender.send(&"user-123".to_string(), 84).unwrap(); // Same receiver as first message
//!
//!     // Receive messages from different consumers
//!     for receiver in &mut receivers {
//!         if let Some(message) = receiver.try_recv().ok() {
//!             println!("Received: {}", message);
//!         }
//!     }
//! }
//! ```
//!
//! ## Bounded Sticky Channel
//!
//! ```rust
//! use tokio_sticky_channel::sticky_channel;
//! use std::num::NonZeroUsize;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create a bounded sticky channel with 3 consumers and capacity of 100 per channel
//!     let (sender, mut receivers) = sticky_channel::<String, i32>(
//!         NonZeroUsize::new(3).unwrap(),
//!         100
//!     );
//!
//!     // Send messages with IDs - will block if target channel is full
//!     sender.send(&"user-123".to_string(), 42).await.unwrap();
//!     sender.send(&"user-456".to_string(), 24).await.unwrap();
//!
//!     // Try to send without blocking - returns error if channel is full
//!     match sender.try_send(&"user-789".to_string(), 99) {
//!         Ok(_) => println!("Message sent successfully"),
//!         Err(e) => println!("Failed to send: {}", e),
//!     }
//!
//!     // Drop sender to close channels
//!     drop(sender);
//!
//!     // Receive messages from all receivers
//!     for receiver in &mut receivers {
//!         while let Some(message) = receiver.recv().await {
//!             println!("Received: {}", message);
//!         }
//!     }
//! }
//! ```
//!
//! # Architecture
//!
//! The sticky channel uses consistent hashing to route messages:
//!
//! 1. **Senders**: Compute `hash(id) % num_consumers` to determine the target receiver
//! 2. **Internal channels**: Each consumer has its own MPSC channel (bounded or unbounded)
//! 3. **Receivers**: Wrap Tokio's receivers with additional convenience methods
//!
//! # Performance Considerations
//!
//! - **Unbounded channels**: Memory usage can grow if consumers can't keep up
//! - **Bounded channels**: Provide backpressure but may block senders when full
//! - **Hashing overhead**: Each send operation computes a hash of the ID
//! - **Load distribution**: Hash distribution may not be perfectly even across consumers

mod bounded;
mod error;
mod unbounded;

#[cfg(test)]
mod tests;

pub use self::{
    bounded::{Receiver, Sender, sticky_channel},
    error::{SendError, TryRecvError},
    unbounded::{UnboundedReceiver, UnboundedSender, unbounded_sticky_channel},
};
