//! Integration of TUN/TAP into tokio.
//!
//! See the [`Async`](struct.Async.html) structure.

use futures::ready;
use std::io::{self, Result};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::unix::AsyncFd;
use tokio::io::{AsyncRead, AsyncWrite};

use super::Iface;

/// A wrapper around [`Iface`](../struct.Iface.html) for use in connection with tokio.
///
/// It implements AsyncWrite and AsyncRead
pub struct Async {
    inner: AsyncFd<Iface>,
}

impl Async {
    /// Consumes an `Iface` and wraps it in a new `Async`.
    ///
    /// # Parameters
    ///
    /// * `iface`: The created interface to wrap. It gets consumed.
    ///
    /// # Errors
    ///
    /// This fails with an error in case of low-level OS errors (they shouldn't usually happen).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use tun_tap::r#async::Async;
    /// use tun_tap::{Iface, Mode};
    ///
    /// use tokio_util::codec::{Decoder, Encoder};
    /// use futures_util::stream::StreamExt;
    /// use bytes::BytesMut;
    ///
    /// struct Frame {
    ///   data: Vec<u8>
    /// }
    ///
    /// struct Codec;
    /// impl Decoder for Codec {
    ///     type Item = Frame;
    ///     type Error = std::io::Error;
    ///     // decoding data from buffer into frames
    ///     fn decode(&mut self, _: &mut BytesMut) -> Result<Option<Frame>, std::io::Error>
    ///         { todo!() }
    /// }
    ///
    /// impl Encoder<Frame> for Codec {
    ///     type Error = std::io::Error;
    ///     // encoding frame into buffer
    ///     fn encode(&mut self, _: Frame, _: &mut BytesMut) -> Result<(), std::io::Error>
    ///        { todo!() }
    /// }
    /// #
    /// # fn main() {
    /// let iface = Iface::new("mytun%d", Mode::Tun).unwrap();
    /// let iface = Async::new(iface).unwrap();
    /// let (sink, stream) = Codec.framed(iface).split();
    /// # }
    /// ```
    pub fn new(iface: Iface) -> Result<Self> {
        iface.set_non_blocking()?;
        Ok(Async {
            inner: AsyncFd::new(iface)?,
        })
    }

    /// Receives a packet from the interface.
    pub async fn recv(&self, out: &mut [u8]) -> io::Result<usize> {
        loop {
            let mut guard = self.inner.readable().await?;

            match guard.try_io(|inner| inner.get_ref().recv(out)) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }
    /// Sends a packet into the interface.
    pub async fn send(&mut self, buf: &[u8]) -> Result<usize> {
        loop {
            let mut guard = self.inner.writable().await?;

            match guard.try_io(|inner| inner.get_ref().send(buf)) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }
}

impl AsyncRead for Async {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        loop {
            let mut guard = ready!(self.inner.poll_read_ready(cx))?;

            let unfilled = buf.initialize_unfilled();
            match guard.try_io(|inner| inner.get_ref().recv(unfilled)) {
                Ok(Ok(len)) => {
                    buf.advance(len);
                    return Poll::Ready(Ok(()));
                }
                Ok(Err(err)) => return Poll::Ready(Err(err)),
                Err(_would_block) => continue,
            }
        }
    }
}

impl AsyncWrite for Async {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::result::Result<usize, io::Error>> {
        loop {
            let mut guard = ready!(self.inner.poll_write_ready(cx))?;

            match guard.try_io(|inner| inner.get_ref().send(buf)) {
                Ok(result) => return Poll::Ready(result),
                Err(_would_block) => continue,
            }
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), io::Error>> {
        // not sure what this method should do
        Poll::Ready(Ok(()))
    }
}
