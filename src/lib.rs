use std::ffi::CStr;
use std::fs::{File, OpenOptions};
use std::io::{Error, Read, Result, Write};
use std::os::raw::{c_char, c_int};
use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};

extern "C" {
    fn tuntap_setup(fd: c_int, name: *mut u8, mode: c_int) -> c_int;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Mode {
    Tun = 1,
    Tap = 2,
}

#[derive(Debug)]
pub struct Iface {
    fd: File,
    mode: Mode,
    name: String,
}

impl Iface {
    pub fn new(ifname: &str, mode: Mode) -> Result<Self> {
        let fd = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/net/tun")?;
        // The buffer is larger than needed, but who cares… it is large enough.
        let mut name_buffer = Vec::new();
        name_buffer.extend_from_slice(ifname.as_bytes());
        name_buffer.extend_from_slice(&[0; 33]);
        let name_ptr: *mut u8 = name_buffer.as_mut_ptr();
        let result = unsafe { tuntap_setup(fd.as_raw_fd(), name_ptr, mode as c_int) };
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
    pub fn mode(&self) -> Mode {
        self.mode
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Read for Iface {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.fd.read(buf)
    }
}

impl<'a> Read for &'a Iface {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        (&self.fd).read(buf)
    }
}

impl Write for Iface {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.fd.write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        self.fd.flush()
    }
}

impl<'a> Write for &'a Iface {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (&self.fd).write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        (&self.fd).flush()
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
