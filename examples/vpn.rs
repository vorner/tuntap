//! An VPN example
//!
//! This creates one endpoint of a VPN. It takes two parameters ‒ local address and the address
//! address of the other endpoint, and sends all packets there, encapsulated in UDP. Packets
//! received from the other side are put to the kernel from the other side.
//!
//! Unlike the other examples, this doesn't configure the kernel endpoint and it is left up for the
//! caller to bring the interface up and add an address to it (or possibly some routes).
//!
//! # Warning
//!
//! Do not use as a VPN in any real-life situation. There's no authentication, encryption, nearly
//! no error handling, etc.

extern crate futures;
extern crate tokio_core;
extern crate tun_tap;

use std::env;
use std::io::Result;
use std::net::SocketAddr;

use futures::{Future, Stream};
use tokio_core::net::{UdpCodec, UdpSocket};
use tokio_core::reactor::Core;

use tun_tap::{Iface, Mode};
use tun_tap::async::Async;

struct VecCodec(SocketAddr);

impl UdpCodec for VecCodec {
    type In = Vec<u8>;
    type Out = Vec<u8>;
    fn decode(&mut self, _src: &SocketAddr, buf: &[u8]) -> Result<Self::In> {
        Ok(buf.to_owned())
    }
    fn encode(&mut self, msg: Self::Out, buf: &mut Vec<u8>) -> SocketAddr {
        buf.extend(&msg);
        self.0
    }
}

fn main() {
    let mut core = Core::new().unwrap();
    let loc_address = env::args()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let rem_address = env::args()
        .nth(2)
        .unwrap()
        .parse()
        .unwrap();
    let socket = UdpSocket::bind(&loc_address, &core.handle())
        .unwrap();
    let (sender, receiver) = socket.framed(VecCodec(rem_address))
        .split();
    let tun = Iface::new("vpn%d", Mode::Tun)
        .unwrap();
    let (sink, stream) = Async::new(tun, &core.handle())
        .unwrap()
        .split();
    let reader = stream.forward(sender);
    let writer = receiver.forward(sink);
    core.run(reader.join(writer))
        .unwrap();
}
