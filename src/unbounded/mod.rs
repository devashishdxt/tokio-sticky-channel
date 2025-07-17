mod receiver;
mod sender;

pub use self::{receiver::UnboundedReceiver, sender::UnboundedSender};

use std::{hash::Hash, num::NonZeroUsize};

/// Creates a sticky channel with the specified number of consumers.
///
/// This function returns a tuple containing a [`UnboundedSender`] and a vector of [`UnboundedReceiver`]s.
///
/// The [`UnboundedSender`] can be used to send messages to the consumers, and each [`UnboundedReceiver`] can be used to receive messages.
///
/// Each message sent via the [`UnboundedSender`] will be delivered to one of the [`UnboundedReceiver`]s in a deterministic manner based
/// on the hash of the ID provided to the [`send`](UnboundedSender::send) method.
pub fn unsticky_channel<ID, T>(
    num_consumers: NonZeroUsize,
) -> (UnboundedSender<ID, T>, Vec<UnboundedReceiver<T>>)
where
    ID: Hash,
{
    let mut receivers = Vec::with_capacity(num_consumers.get());
    let mut sender = UnboundedSender {
        consumers: Vec::with_capacity(num_consumers.get()),
        _phantom: std::marker::PhantomData,
    };

    for _ in 0..num_consumers.get() {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        sender.consumers.push(tx);
        receivers.push(UnboundedReceiver { receiver: rx });
    }

    (sender, receivers)
}
