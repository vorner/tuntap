# TunTap

[![Travis Build Status](https://api.travis-ci.org/vorner/tuntap.png?branch=master)](https://travis-ci.org/vorner/tuntap)

TUN/TAP wrapper for Rust.

The TUN/TAP allows implementing a virtual network adapter in userspace. This
provides the bindings for Rust.

Create an `Iface` object and `send` or `recv` packets. Making some sense of the
packets is, however, out of scope, you need something else for that.

There's [documentation](https://docs.rs/tun-tap) and some
[examples](https://github.com/vorner/tuntap/tree/master/examples).

## Known issues

* Tested only on Linux. Probably doesn't work anywhere else, but pull requests
  adding support for other OSes are welcome.
* The asynchronous interface is very minimal and probably inefficient. It'll
  need to be extended to allow more flexible or efficient use.

## Testing

In order to run the tests correctly, a `tun10` device with `10.10.10.1/24` needs
to be created first. You can simply run:

```shell
sh tests/setup.sh
```

To remove it:

```shell
sh tests/clean.sh
```

Sudo permission is required.

## MSRV
The current MSRV(minimum supported rust version) is 1.60.

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms
or conditions.
