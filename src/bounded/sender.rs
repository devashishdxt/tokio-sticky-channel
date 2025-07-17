use std::{
    hash::{DefaultHasher, Hash, Hasher},
    num::TryFromIntError,
};

use tokio::sync::mpsc::Sender as MpscSender;

use crate::SendError;

/// Send values to the associated [`Receiver`](crate::Receiver).
#[derive(Clone)]
pub struct Sender<ID, T> {
    pub(crate) consumers: Vec<MpscSender<T>>,
    pub(crate) _phantom: std::marker::PhantomData<ID>,
}

impl<ID, T> Sender<ID, T>
where
    ID: core::hash::Hash,
{
    /// Attempts to send a message to the consumer identified by `id`.
    ///
    /// This method will block if the target channel is at capacity until space becomes available.
    ///
    /// If the receive half of the channel is closed, either due to [`close`](crate::Receiver::close) being called or
    /// the [`Receiver`](crate::Receiver) having been dropped, this function returns an error. The error includes the
    /// value passed to `send`.
    pub async fn send(&self, id: &ID, message: T) -> Result<(), SendError<T>> {
        match compute_route_id(id, self.consumers.len()) {
            Ok(route_id) => match self.consumers.get(route_id) {
                Some(sender) => sender
                    .send(message)
                    .await
                    .map_err(|err| SendError::ChannelClosed(err.0)),
                None => Err(SendError::NoConsumer(message)),
            },
            Err(_) => Err(SendError::FailedToComputeRouteID(message)),
        }
    }

    /// Attempts to send a message to the consumer identified by `id` without blocking.
    ///
    /// This method will return an error if the target channel is at capacity.
    ///
    /// If the receive half of the channel is closed, either due to [`close`](crate::Receiver::close) being called or
    /// the [`Receiver`](crate::Receiver) having been dropped, this function returns an error. The error includes the
    /// value passed to `try_send`.
    pub fn try_send(&self, id: &ID, message: T) -> Result<(), SendError<T>> {
        match compute_route_id(id, self.consumers.len()) {
            Ok(route_id) => match self.consumers.get(route_id) {
                Some(sender) => sender.try_send(message).map_err(|err| match err {
                    tokio::sync::mpsc::error::TrySendError::Full(msg) => {
                        SendError::ChannelFull(msg)
                    }
                    tokio::sync::mpsc::error::TrySendError::Closed(msg) => {
                        SendError::ChannelClosed(msg)
                    }
                }),
                None => Err(SendError::NoConsumer(message)),
            },
            Err(_) => Err(SendError::FailedToComputeRouteID(message)),
        }
    }
}

fn compute_route_id<ID>(id: &ID, num_consumers: usize) -> Result<usize, TryFromIntError>
where
    ID: Hash,
{
    let mut hasher = DefaultHasher::new();
    id.hash(&mut hasher);
    let hash = usize::try_from(hasher.finish())?;

    Ok(hash % num_consumers)
}
