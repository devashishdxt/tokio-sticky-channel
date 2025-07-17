# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust crate that implements a sticky channel pattern for asynchronous message passing using Tokio. The channel routes messages to specific receivers based on a hash of an ID, ensuring that all messages with the same ID are consistently delivered to the same receiver. The crate provides both bounded and unbounded channel variants.

## Common Development Commands

### Build
```bash
cargo build
cargo build --release
```

### Test
```bash
cargo test
cargo test -- --nocapture  # Show println! output during tests
```

### Check and Lint
```bash
cargo check
cargo clippy
cargo fmt -- --check  # Check formatting without changing files
cargo fmt            # Auto-format code
```

### Documentation
```bash
cargo doc --open     # Build and open documentation in browser
cargo doc --no-deps  # Build docs without dependencies
```

## Architecture Overview

The crate consists of several main components:

1. **lib.rs**: Main entry point and public API exports
   - Exports both bounded and unbounded channel APIs
   - Contains crate-level documentation with examples

2. **bounded/mod.rs**: Factory function for creating bounded sticky channels
   - Implements `sticky_channel()` function
   - Creates bounded MPSC channels with specified capacity

3. **bounded/sender.rs**: Bounded channel sender implementation
   - `Sender` struct with async `send()` and non-blocking `try_send()` methods
   - Routes messages using consistent hashing (`hash % num_consumers`)
   - Provides backpressure when channels are full

4. **bounded/receiver.rs**: Wraps Tokio's bounded Receiver
   - `Receiver` struct with async (`recv`, `recv_many`) and sync (`try_recv`) methods
   - Includes `close()` method for graceful shutdown
   - All methods are cancel-safe for use with `tokio::select!`

5. **unbounded/mod.rs**: Factory function for creating unbounded sticky channels
   - Implements `unsticky_channel()` function
   - Creates unbounded MPSC channels with no capacity limit

6. **unbounded/sender.rs**: Unbounded channel sender implementation
   - `UnboundedSender` struct with synchronous `send()` method
   - Routes messages using the same consistent hashing algorithm
   - No backpressure - messages are always accepted

7. **unbounded/receiver.rs**: Wraps Tokio's UnboundedReceiver
   - `UnboundedReceiver` struct with async (`recv`, `recv_many`) and sync (`try_recv`) methods
   - All methods are cancel-safe for use with `tokio::select!`

8. **error.rs**: Error type definitions
   - `SendError` for send operation failures (includes `ChannelFull` variant for bounded channels)
   - `TryRecvError` for non-blocking receive failures

### Key Design Decisions

- **Fixed Consumer Count**: Number of consumers is determined at channel creation and cannot be changed
- **Channel Types**: 
  - **Bounded channels**: Provide backpressure and memory bounds via configurable capacity
  - **Unbounded channels**: No capacity limits but potential for unbounded memory growth
- **Deterministic Routing**: Same ID always routes to the same consumer via consistent hashing
- **Clone-able Senders**: Multiple producers can send to the same channel set
- **Async and Sync APIs**: Bounded channels support both blocking (`send`) and non-blocking (`try_send`) operations

### Public API Structure

#### Bounded Channels
```rust
// Create a bounded sticky channel with N consumers and specified capacity
pub fn sticky_channel<ID, T>(num_consumers: NonZeroUsize, capacity: usize) -> (Sender<ID, T>, Vec<Receiver<T>>)

// Send messages with an ID for routing (async, blocks when full)
sender.send(&id, message).await -> Result<(), SendError<T>>

// Try to send without blocking (returns error if full)
sender.try_send(&id, message) -> Result<(), SendError<T>>

// Receive messages (various methods)
receiver.recv().await -> Option<T>
receiver.recv_many(&mut buffer, limit).await -> usize
receiver.try_recv() -> Result<T, TryRecvError>
receiver.close() // Graceful shutdown
```

#### Unbounded Channels
```rust
// Create an unbounded sticky channel with N consumers
pub fn unsticky_channel<ID, T>(num_consumers: NonZeroUsize) -> (UnboundedSender<ID, T>, Vec<UnboundedReceiver<T>>)

// Send messages with an ID for routing (sync, never blocks)
sender.send(&id, message) -> Result<(), SendError<T>>

// Receive messages (various methods)
receiver.recv().await -> Option<T>
receiver.recv_many(&mut buffer, limit).await -> usize
receiver.try_recv() -> Result<T, TryRecvError>
```

## Dependencies

- **tokio**: Async runtime with "sync" feature for channels
- **thiserror**: Error handling and derive macros