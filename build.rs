extern crate cc;

use cc::Build;

fn main() {
    Build::new()
        .file("src/tuntap.c")
        .warnings(true)
        .compile("tuntap");
}
