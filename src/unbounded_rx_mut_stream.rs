// Modified from https://docs.rs/tokio-stream/0.1.18/src/tokio_stream/wrappers/mpsc_unbounded.rs.html#34-36

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use tokio::sync::mpsc::UnboundedReceiver;

/// A wrapper around [`&mut tokio::sync::mpsc::UnboundedReceiver`] that implements [`Stream`].
///
/// # Example
///
/// ```
/// use tokio::sync::mpsc;
/// use tokio_stream::wrappers::UnboundedReceiverStream;
/// use tokio_stream::StreamExt;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), tokio::sync::mpsc::error::SendError<u8>> {
/// let (tx, rx) = mpsc::unbounded_channel();
/// tx.send(10)?;
/// tx.send(20)?;
/// # // prevent the doc test from hanging
/// drop(tx);
///
/// let mut stream = UnboundedReceiverStream::new(rx);
/// assert_eq!(stream.next().await, Some(10));
/// assert_eq!(stream.next().await, Some(20));
/// assert_eq!(stream.next().await, None);
/// # Ok(())
/// # }
/// ```
///
/// [`tokio::sync::mpsc::UnboundedReceiver`]: struct@tokio::sync::mpsc::UnboundedReceiver
/// [`Stream`]: trait@crate::Stream
#[derive(Debug)]
pub struct UnboundedReceiverMutStream<'a, T> {
    inner: &'a mut UnboundedReceiver<T>,
}

impl<'a, T> UnboundedReceiverMutStream<'a, T> {
    /// Create a new `UnboundedReceiverStream`.
    pub fn new(recv: &'a mut UnboundedReceiver<T>) -> Self {
        Self { inner: recv }
    }
}

impl<'a, T> Stream for UnboundedReceiverMutStream<'a, T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.poll_recv(cx)
    }

    /// Returns the bounds of the stream based on the underlying receiver.
    ///
    /// For open channels, it returns `(receiver.len(), None)`.
    ///
    /// For closed channels, it returns `(receiver.len(), receiver.len())`.
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.inner.is_closed() {
            let len = self.inner.len();
            (len, Some(len))
        } else {
            (self.inner.len(), None)
        }
    }
}

impl<'a, T> From<&'a mut UnboundedReceiver<T>> for UnboundedReceiverMutStream<'a, T> {
    fn from(recv: &'a mut UnboundedReceiver<T>) -> Self {
        Self::new(recv)
    }
}
