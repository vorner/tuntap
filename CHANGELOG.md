# 0.2.0

## Breaking Changes

* Async API completely rewritten: The `Async` struct no longer implements
  `Stream` or `Sink`. It simply provides async version `recv` and `send` methods for reading/writing packets.
* `set_recv_bufsize` removed from `Async`; external buffer sizing is now caller-managed
  via `AsyncRead`.
* Rename module `r#async` to `aio` to avoid conflict with Rust 2024 edition's `async` keyword.

## Dependency Upgrades

* `tokio-core` 0.1 → `tokio` 1.x (features: net, rt, rt-multi-thread, io-util, macros, time)
* `futures` 0.1 → removed
* `mio` 0.6 → removed (tokio's `AsyncFd` replaces raw mio)
* `etherparse` 0.9 → 0.17 (dev-dependency)
* `serial_test` 0.4 → 3.x (dev-dependency)

## Upstream Fixes (since 0.1.4)

* Fixed `cmd` function in examples to accept arbitrary command instead of hardcoded `"ip"` (#17)
* Fixed typo in doc comment (#16)

# 0.1.4

* Ability to set nonblocking without tokio (#12).

# 0.1.3

* Add MacOS support (#10).

# 0.1.2

* The `without_packet_info` constructor.

# 0.1.1

* Fixes in documentation links.
* No real code changes.

# 0.1.0

Initial implementation:
* Ability to open TUN/TAP device and send/receive packets.
* An async wrapper for integration with tokio.
