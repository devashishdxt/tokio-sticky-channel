use std::hash::{BuildHasher, Hash, RandomState};

use tokio::sync::mpsc::UnboundedSender as MpscSender;

use crate::{SendError, util::compute_route_id};

/// Send values to the associated [`UnboundedReceiver`](crate::UnboundedReceiver).
#[derive(Clone)]
pub struct UnboundedSender<ID, T, S = RandomState> {
    pub(crate) consumers: Vec<MpscSender<T>>,
    pub(crate) build_hasher: S,
    pub(crate) _phantom: std::marker::PhantomData<ID>,
}

impl<ID, T, S> UnboundedSender<ID, T, S>
where
    ID: Hash,
    S: BuildHasher,
{
    /// Attempts to send a message to the consumer identified by `id` without blocking.
    ///
    /// This method is not marked async because sending a message to a channel never requires any form of waiting.
    /// Because of this, the `send` method can be used in both synchronous and asynchronous code without problems.
    ///
    /// If the receive half of the channel is closed, either due to [`close`](crate::UnboundedReceiver::close) being called or
    /// the [`UnboundedReceiver`](crate::UnboundedReceiver) having been dropped, this function returns an error. The error includes the
    /// value passed to `send`.
    pub fn send(&self, id: ID, message: T) -> Result<(), SendError<T>> {
        match compute_route_id(id, self.consumers.len(), &self.build_hasher) {
            Ok(route_id) => match self.consumers.get(route_id) {
                Some(sender) => sender
                    .send(message)
                    .map_err(|err| SendError::ChannelClosed(err.0)),
                None => Err(SendError::NoConsumer(message)),
            },
            Err(_) => Err(SendError::FailedToComputeRouteID(message)),
        }
    }
}
