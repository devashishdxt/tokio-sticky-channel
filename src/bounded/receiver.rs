use tokio::sync::mpsc::Receiver as MpscReceiver;

use crate::TryRecvError;

/// Receive values from the associated [`Sender`](crate::Sender).
pub struct Receiver<T> {
    pub(crate) receiver: MpscReceiver<T>,
}

impl<T> Receiver<T> {
    /// Receives the next message for this receiver.
    ///
    /// This method returns `None` if the channel has been closed and there are no remaining messages in the channel's
    /// buffer. This indicates that no further values can ever be received from this `Receiver`. The channel is closed
    /// when all senders have been dropped, or when [`close`](Receiver::close) is called.
    ///
    /// If there are no messages in the channel's buffer, but the channel has not yet been closed, this method will
    /// sleep until a message is sent or the channel is closed.
    ///
    /// # Cancel safety
    ///
    /// This method is cancel safe. If `recv` is used as the event in a `tokio::select!` statement and some other branch
    /// completes first, it is guaranteed that no messages were received on this channel.
    pub async fn recv(&mut self) -> Option<T> {
        self.receiver.recv().await
    }

    /// Receives the next messages for this receiver and extends `buffer`.
    ///
    /// This method extends `buffer` by no more than a fixed number of values as specified by `limit`. If `limit` is
    /// zero, the function returns immediately with `0`. The return value is the number of values added to `buffer`.
    ///
    /// For `limit > 0`, if there are no messages in the channel's queue, but the channel has not yet been closed, this
    /// method will sleep until a message is sent or the channel is closed.
    ///
    /// For non-zero values of `limit`, this method will never return `0` unless the channel has been closed and there
    /// are no remaining messages in the channel's queue. This indicates that no further values can ever be received
    /// from this `Receiver`. The channel is closed when all senders have been dropped, or when
    /// [`close`](Receiver::close) is called.
    ///
    /// The capacity of `buffer` is increased as needed.
    ///
    /// # Cancel safety
    ///
    /// This method is cancel safe. If `recv_many` is used as the event in a `tokio::select!` statement and some other
    /// branch completes first, it is guaranteed that no messages were received on this channel.
    pub async fn recv_many(&mut self, buffer: &mut Vec<T>, limit: usize) -> usize {
        self.receiver.recv_many(buffer, limit).await
    }

    /// Tries to receive the next message for this receiver.
    ///
    /// This method returns the [`Empty`](TryRecvError::Empty) error if the channel is currently empty, but there are still outstanding
    /// [`Sender`](crate::Sender).
    ///
    /// This method returns the [`Disconnected`](TryRecvError::Disconnected) error if the channel is currently empty,
    /// and there are no outstanding [`Sender`](crate::Sender).
    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        self.receiver.try_recv().map_err(|err| match err {
            tokio::sync::mpsc::error::TryRecvError::Empty => TryRecvError::Empty,
            tokio::sync::mpsc::error::TryRecvError::Disconnected => TryRecvError::Disconnected,
        })
    }

    /// Closes the receiver without dropping it.
    ///
    /// This prevents any further messages from being sent on the channel while still enabling the receiver to drain
    /// messages that are buffered.
    ///
    /// To guarantee that no messages are dropped, after calling `close()`, `recv()` must be called until `None` is
    /// returned.
    pub fn close(&mut self) {
        self.receiver.close();
    }
}