//! An example of reading from tun
//!
//! It creates a tun device, sets it up (using shell commands) for local use and then prints the
//! raw data of the packets that arrive.
//!
//! You really do want better error handling than all these unwraps.
extern crate tun_tap;

use std::process::Command;

use tun_tap::{Iface, Mode};

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
    // Create the tun interface.
    let iface = Iface::new("testtun%d", Mode::Tun).unwrap();
    eprintln!("Iface: {:?}", iface);
    // Configure the „local“ (kernel) endpoint.
    cmd("ip", &["addr", "add", "dev", iface.name(), "10.107.1.2/24"]);
    cmd("ip", &["link", "set", "up", "dev", iface.name()]);
    println!("Created interface {}. Send some packets into it and see they're printed here",
             iface.name());
    println!("You can for example ping 10.107.1.3 (it won't answer)");
    // That 1500 is a guess for the IFace's MTU (we probably could configure it explicitly). 4 more
    // for TUN's „header“.
    let mut buffer = vec![0; 1504];
    loop {
        // Every read is one packet. If the buffer is too small, bad luck, it gets truncated.
        let size = iface.recv(&mut buffer).unwrap();
        assert!(size >= 4);
        println!("Packet: {:?}", &buffer[4..size]);
    }
}
