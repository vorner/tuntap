#![doc(
    html_root_url = "https://docs.rs/tun-tap/0.1.2/tun-tap/",
    test(attr(deny(warnings), allow(unused_variables)))
)]
#![deny(missing_docs)]

//! A TUN/TAP bindings for Rust.
//!
//! This is a basic interface to create userspace virtual network adapter.
//!
//! For basic usage, create an [`Iface`](struct.Iface.html) object and call the
//! [`send`](struct.Iface.html#method.send) and [`recv`](struct.Iface.html#method.recv) methods.
//!
//! You can also use [`Async`](async/struct.Async.html) if you want to integrate with tokio event
//! loop. This is configurable by a feature (it is on by default).
//!
//! Creating the devices requires `CAP_NETADM` privileges (most commonly done by running as root).
//!
//! # Known issues
//!
//! * It is tested only on Linux and probably doesn't work anywhere else, even though other systems
//!   have some TUN/TAP support. Reports that it works (or not) and pull request to add other
//!   sustem's support are welcome.
//! * The [`Async`](async/struct.Async.html) interface is very minimal and will require extention
//!   for further use cases and better performance.
//! * This doesn't support advanced usage patters, like reusing already created device or creating
//!   persistent devices. Again, pull requests are welcome.
//! * There are no automated tests. Any idea how to test this in a reasonable way?

use std::ffi::CStr;
use std::fs::{File, OpenOptions};
use std::io::{Error, Read, Result, Write};
use std::os::raw::{c_char, c_int};
use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};

#[cfg(feature = "tokio")]
pub mod async;

extern "C" {
    fn tuntap_setup(fd: c_int, name: *mut u8, mode: c_int, packet_info: c_int) -> c_int;
}

/// The mode in which open the virtual network adapter.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Mode {
    /// TUN mode
    ///
    /// The packets returned are on the IP layer (layer 3), prefixed with 4-byte header (2 bytes
    /// are flags, 2 bytes are the protocol inside, eg one of
    /// <https://en.wikipedia.org/wiki/EtherType#Examples>.
    Tun = 1,
    /// TAP mode
    ///
    /// The packets are on the transport layer (layer 2), and start with ethernet frame header.
    Tap = 2,
}

/// The virtual interface.
///
/// This is the main structure of the crate, representing the actual virtual interface, either in
/// TUN or TAP mode.
#[derive(Debug)]
pub struct Iface {
    fd: File,
    mode: Mode,
    name: String,
}

impl Iface {
    /// Creates a new virtual interface.
    ///
    /// # Parameters
    ///
    /// * `ifname`: The requested name of the virtual device. If left empty, the kernel will
    ///   provide some reasonable, currently unused name. It also can contain `%d`, which will be
    ///   replaced by a number to ensure the name is unused. Even if it isn't empty or doesn't
    ///   contain `%d`, the actual name may be different (for example truncated to OS-dependent
    ///   length). Use [`name`](#method.name) to find out the real name.
    /// * `mode`: In which mode to create the device.
    ///
    /// # Errors
    ///
    /// This may fail for various OS-dependent reasons. However, two most common are:
    ///
    /// * The name is already taken.
    /// * The process doesn't have the needed privileges (eg. `CAP_NETADM`).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use tun_tap::*;
    /// let iface = Iface::new("mytun", Mode::Tun).expect("Failed to create a TUN device");
    /// let name = iface.name();
    /// // Configure the device ‒ set IP address on it, bring it up.
    /// let mut buffer = vec![0; 1504]; // MTU + 4 for the header
    /// iface.recv(&mut buffer).unwrap();
    /// ```
    pub fn new(ifname: &str, mode: Mode) -> Result<Self> {
        Iface::with_options(ifname, mode, true)
    }
    /// Creates a new virtual interface without the prepended packet info.
    ///
    /// # Parameters
    ///
    /// * `ifname`: The requested name of the virtual device. If left empty, the kernel will
    ///   provide some reasonable, currently unused name. It also can contain `%d`, which will be
    ///   replaced by a number to ensure the name is unused. Even if it isn't empty or doesn't
    ///   contain `%d`, the actual name may be different (for example truncated to OS-dependent
    ///   length). Use [`name`](#method.name) to find out the real name.
    /// * `mode`: In which mode to create the device.
    ///
    /// # Errors
    ///
    /// This may fail for various OS-dependent reasons. However, two most common are:
    ///
    /// * The name is already taken.
    /// * The process doesn't have the needed privileges (eg. `CAP_NETADM`).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use tun_tap::*;
    /// let iface = Iface::without_packet_info("mytap", Mode::Tap).expect("Failed to create a TAP device");
    /// let name = iface.name();
    /// // Configure the device ‒ set IP address on it, bring it up.
    /// let mut buffer = vec![0; 1500]; // MTU
    /// iface.recv(&mut buffer).unwrap();
    /// ```
    pub fn without_packet_info(ifname: &str, mode: Mode) -> Result<Self> {
        Iface::with_options(ifname, mode, false)
    }

    fn with_options(ifname: &str, mode: Mode, packet_info: bool) -> Result<Self> {
        let fd = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/net/tun")?;
        // The buffer is larger than needed, but who cares… it is large enough.
        let mut name_buffer = Vec::new();
        name_buffer.extend_from_slice(ifname.as_bytes());
        name_buffer.extend_from_slice(&[0; 33]);
        let name_ptr: *mut u8 = name_buffer.as_mut_ptr();
        let result = unsafe { tuntap_setup(fd.as_raw_fd(), name_ptr, mode as c_int, { if packet_info { 1 } else { 0 } }) };
        if result < 0 {
            return Err(Error::last_os_error());
        }
        let name = unsafe {
            CStr::from_ptr(name_ptr as *const c_char)
                .to_string_lossy()
                .into_owned()
        };
        Ok(Iface {
            fd,
            mode,
            name,
        })
    }

    /// Returns the mode of the adapter.
    ///
    /// It is always the same as the one passed to [`new`](#method.new).
    pub fn mode(&self) -> Mode {
        self.mode
    }
    /// Returns the real name of the adapter.
    ///
    /// Use this to find out what the real name of the adapter is. The parameter of
    /// [`new`](#method.new) is more of a wish than hard requirement and the name of the created
    /// device might be different. Therefore, always create the interface and then find out the
    /// actual name by this method before proceeding.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Receives a packet from the interface.
    ///
    /// Blocks until a packet is sent into the virtual interface. At that point, the content of the
    /// packet is copied into the provided buffer.
    ///
    /// Make sure the buffer is large enough. It is MTU of the interface (usually 1500, unless
    /// reconfigured) + 4 for the header in case that packet info is prepended, MTU + size of ethernet frame (38 bytes,
    /// unless VLan tags are enabled). If the buffer isn't large enough, the packet gets truncated.
    ///
    /// # Result
    ///
    /// On successful receive, the number of bytes copied into the buffer is returned.
    pub fn recv(&self, buf: &mut [u8]) -> Result<usize> {
        (&self.fd).read(buf)
    }
    /// Sends a packet into the interface.
    ///
    /// Sends a packet through the interface. The buffer must be valid representation of a packet
    /// (with appropriate headers).
    ///
    /// It is up to the caller to provide only packets that fit MTU.
    ///
    /// # Result
    ///
    /// On successful send, the number of bytes sent in the packet is returned. Under normal
    /// circumstances, this should be the size of the packet passed.
    ///
    /// # Notes
    ///
    /// The TUN/TAP is a network adapter. Therefore, many errors are handled simply by dropping
    /// packets. If you pass an invalid packet, it'll likely suceed in sending it, but will be
    /// dropped somewhere in kernel due to failed validation. If you send packets too fast, they
    /// are likely to get dropped too. If you send a packet for address that is not assigned to any
    /// interface and not routed anywhere… you get the idea.
    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        (&self.fd).write(buf)
    }
}

impl AsRawFd for Iface {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl IntoRawFd for Iface {
    fn into_raw_fd(self) -> RawFd {
        self.fd.into_raw_fd()
    }
}
