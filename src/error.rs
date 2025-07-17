/// Error type for receiving messages through [`UnboundedReceiver::try_recv`](crate::UnboundedReceiver::try_recv) and [`Receiver::try_recv`](crate::Receiver::try_recv).
#[derive(Debug, thiserror::Error)]
pub enum TryRecvError {
    /// The channel is empty.
    #[error("channel is empty")]
    Empty,

    /// The channel is disconnected.
    #[error("channel is disconnected")]
    Disconnected,
}

/// Error type for sending messages through the [`UnboundedSender::send`](crate::UnboundedSender::send) and [`Sender::send`](crate::Sender::send).
#[derive(Debug, thiserror::Error)]
#[error("channel closed")]
pub enum SendError<T> {
    /// The message could not be send because there is no receiver for given ID.
    #[error("no receiver for ID")]
    NoConsumer(T),

    /// The channel was closed before the message could be sent.
    #[error("channel closed")]
    ChannelClosed(T),

    /// The channel is full and cannot accept more messages (bounded channels only).
    #[error("channel is full")]
    ChannelFull(T),

    /// Failed to compute route ID from the given ID.
    #[error("failed to compute route ID")]
    FailedToComputeRouteID(T),
}
