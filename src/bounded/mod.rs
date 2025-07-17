mod receiver;
mod sender;

pub use self::{receiver::Receiver, sender::Sender};

use std::{hash::Hash, num::NonZeroUsize};

/// Creates a bounded sticky channel with the specified number of consumers and capacity.
///
/// This function returns a tuple containing a [`Sender`] and a vector of [`Receiver`]s.
///
/// The [`Sender`] can be used to send messages to the consumers, and each [`Receiver`] can be used to receive messages.
///
/// Each message sent via the [`Sender`] will be delivered to one of the [`Receiver`]s in a deterministic manner based
/// on the hash of the ID provided to the [`send`](Sender::send) method.
///
/// Each internal channel will have the specified capacity. When a channel is full, sending will block until space becomes available.
pub fn sticky_channel<ID, T>(
    num_consumers: NonZeroUsize,
    capacity: usize,
) -> (Sender<ID, T>, Vec<Receiver<T>>)
where
    ID: Hash,
{
    let mut receivers = Vec::with_capacity(num_consumers.get());
    let mut sender = Sender {
        consumers: Vec::with_capacity(num_consumers.get()),
        _phantom: std::marker::PhantomData,
    };

    for _ in 0..num_consumers.get() {
        let (tx, rx) = tokio::sync::mpsc::channel(capacity);
        sender.consumers.push(tx);
        receivers.push(Receiver { receiver: rx });
    }

    (sender, receivers)
}