extern crate gcc;

use gcc::Build;

fn main() {
    Build::new()
        .file("src/tuntap.c")
        .warnings(true)
        .compile("tuntap");
}
