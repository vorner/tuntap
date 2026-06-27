//! Integration of TUN/TAP into tokio.
//!
//! Wraps an [`Iface`] in tokio's [`AsyncFd`] to provide
//! async versions of [`Iface::recv`] and [`Iface::send`].  The underlying
//! fd is automatically set to non-blocking mode so that the kernel
//! returns `EWOULDBLOCK` instead of blocking the reactor thread.

use std::io;

use tokio::io::unix::AsyncFd;

use super::Iface;

/// An asynchronous wrapper around a TUN/TAP [`Iface`].
///
/// Integrates the raw file descriptor with tokio's I/O reactor so that
/// [`recv`](Async::recv) / [`send`](Async::send) can be `.await`ed
/// cooperatively without blocking the runtime.
///
/// The TUN device is **message-oriented** (datagram semantics) —
/// every read/write corresponds to exactly one network packet.
/// Do **not** use `write_all` or byte-stream abstractions; a single
/// `write` that is shorter than the caller intended injects a truncated
/// (garbage) packet into the kernel.
pub struct Async {
    inner: AsyncFd<Iface>,
}

impl Async {
    /// Consumes an `Iface` and wraps it in a new `Async`.
    ///
    /// Sets the underlying fd to non-blocking mode automatically.
    ///
    /// # Errors
    ///
    /// This fails with an error in case of low-level OS errors.
    pub fn new(iface: Iface) -> io::Result<Self> {
        iface.set_non_blocking()?;
        Ok(Async {
            inner: AsyncFd::new(iface)?,
        })
    }
}

impl Async {
    /// Receives a single packet from the interface asynchronously.
    ///
    /// Awaits readability on the underlying fd, then calls
    /// [`Iface::recv`] to copy one packet into `buf`.
    ///
    /// The buffer must be large enough to hold one full packet
    /// (MTU + 4-byte packet-info header if enabled).  If it is
    /// too small the packet is truncated by the kernel.
    ///
    /// # Returns
    ///
    /// The number of bytes written into `buf`.
    pub async fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let mut guard = self.inner.readable().await?;
            match guard.try_io(|inner| inner.get_ref().recv(buf)) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }

    /// Sends a single packet into the interface asynchronously.
    ///
    /// Awaits writability on the underlying fd, then calls
    /// [`Iface::send`] to inject the packet into the kernel
    /// network stack.
    ///
    /// # Returns
    ///
    /// The number of bytes sent (should equal `data.len()` under
    /// normal conditions).
    pub async fn send(&self, data: &[u8]) -> io::Result<usize> {
        loop {
            let mut guard = self.inner.writable().await?;
            match guard.try_io(|inner| inner.get_ref().send(data)) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }
}
