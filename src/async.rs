extern crate futures;
extern crate libc;
extern crate mio;
extern crate tokio_core;

use std::io::{Error, ErrorKind, Read, Result, Write};
use std::os::unix::io::AsRawFd;

use self::futures::{Async as FAsync, AsyncSink, Sink, StartSend, Stream, Poll as FPoll};
use self::libc::c_int;
use self::mio::{Evented, Poll as MPoll, PollOpt, Ready, Token};
use self::mio::unix::EventedFd;
use self::tokio_core::reactor::{Handle, PollEvented};

use super::Iface;

struct MioWrapper {
    iface: Iface,
}

impl Evented for MioWrapper {
    fn register(&self, poll: &MPoll, token: Token, events: Ready, opts: PollOpt) -> Result<()> {
        EventedFd(&self.iface.as_raw_fd()).register(poll, token, events, opts)
    }
    fn reregister(&self, poll: &MPoll, token: Token, events: Ready, opts: PollOpt) -> Result<()> {
        EventedFd(&self.iface.as_raw_fd()).reregister(poll, token, events, opts)
    }
    fn deregister(&self, poll: &MPoll) -> Result<()> {
        EventedFd(&self.iface.as_raw_fd()).deregister(poll)
    }
}

impl Read for MioWrapper {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.iface.recv(buf)
    }
}

impl Write for MioWrapper {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.iface.send(buf)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

pub struct Async {
    mio: PollEvented<MioWrapper>,
    recv_bufsize: usize,
}

impl Async {
    pub fn new(iface: Iface, handle: &Handle) -> Result<Self> {
        let fd = iface.as_raw_fd();
        let mut nonblock: c_int = 1;
        let result = unsafe { libc::ioctl(fd, libc::FIONBIO, &mut nonblock) };
        if result == -1 {
            Err(Error::last_os_error())
        } else {
            Ok(Async {
                mio: PollEvented::new(MioWrapper { iface }, handle)?,
                recv_bufsize: 1504,
            })
        }
    }
    // TODO About the +4
    pub fn set_recv_bufsize(&mut self, bufsize: usize) {
        self.recv_bufsize = bufsize;
    }
}

impl Stream for Async {
    type Item = Vec<u8>;
    type Error = Error;
    fn poll(&mut self) -> FPoll<Option<Self::Item>, Self::Error> {
        // TODO Reuse buffer?
        let mut buffer = vec![0; self.recv_bufsize];
        match self.mio.read(&mut buffer) {
            Ok(size) => {
                buffer.resize(size, 0);
                Ok(FAsync::Ready(Some(buffer)))
            },
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => Ok(FAsync::NotReady),
            Err(e) => Err(e),
        }
    }
}

impl Sink for Async {
    type SinkItem = Vec<u8>;
    type SinkError = Error;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        match self.mio.write(&item) {
            // TODO What to do about short write?
            Ok(_size) => Ok(AsyncSink::Ready),
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => Ok(AsyncSink::NotReady(item)),
            Err(e) => Err(e),
        }
    }
    fn poll_complete(&mut self) -> FPoll<(), Self::SinkError> {
        Ok(FAsync::Ready(()))
    }
}
