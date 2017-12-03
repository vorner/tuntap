//! An example that reads and writes the TUN in parallel.
//!
//! This creates an interface, configures the kernel endpoint and sends pings to that endpoint. It
//! prints whatever packets come back (which should include the pong responses).
//!
//! This does essentialy the same as the `pingpoing` example. However, instead of threads, this
//! uses tokio asynchronous event loop to multiplex between the two directions.
//!
//! You really do want better error handling than all these unwraps.

extern crate futures;
extern crate tokio_core;
extern crate tun_tap;

use std::process::Command;
use std::time::Duration;

use futures::{Future, Stream};
use tokio_core::reactor::{Core, Interval};
use tun_tap::{Iface, Mode};
use tun_tap::async::Async;

/// The packet data. Note that it is prefixed by 4 bytes ‒ two bytes are flags, another two are
/// protocol. 8, 0 is IPv4, 134, 221 is IPv6. <https://en.wikipedia.org/wiki/EtherType#Examples>.
const PING: &[u8] = &[0, 0, 8, 0, 69, 0, 0, 84, 44, 166, 64, 0, 64, 1, 247, 40, 10, 107, 1, 2, 10,
    107, 1, 3, 8, 0, 62, 248, 19, 160, 0, 2, 232, 228, 34, 90, 0, 0, 0, 0, 216, 83, 3, 0, 0, 0, 0,
    0, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38,
    39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55];

/// Run a shell command. Panic if it fails in any way.
fn cmd(cmd: &str, args: &[&str]) {
    let ecode = Command::new("ip")
        .args(args)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    assert!(ecode.success(), "Failed to execte {}", cmd);
}

fn main() {
    let iface = Iface::new("testtun%d", Mode::Tun).unwrap();
    eprintln!("Iface: {:?}", iface);
    // Configure the „local“ (kernel) endpoint. Kernel is (the host) 10.107.1.3, we (the app)
    // pretend to be 10.107.1.2.
    cmd("ip", &["addr", "add", "dev", iface.name(), "10.107.1.3/24"]);
    cmd("ip", &["link", "set", "up", "dev", iface.name()]);
    let mut core = Core::new().unwrap();
    let iface = Async::new(iface, &core.handle()).unwrap();
    let (sink, stream) = iface.split();
    let writer = Interval::new(Duration::from_secs(1), &core.handle())
        .unwrap()
        .map(|_| {
            println!("Sending ping");
            PING.to_owned()
        })
        .forward(sink);
    let reader = stream.for_each(|packet| {
        println!("Received: {:?}", packet);
        Ok(())
    });
    core.run(reader.join(writer)).unwrap();
}
