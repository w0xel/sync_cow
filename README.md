# sync_cow

[![Crates.io](https://img.shields.io/crates/v/sync_cow)](https://crates.io/crates/sync_cow)
[![Docs.rs](https://docs.rs/sync_cow/badge.svg)](https://docs.rs/sync_cow)
[![CI](https://github.com/w0xel/sync_cow/actions/workflows/rust.yml/badge.svg)](https://github.com/w0xel/sync_cow/actions)
![Crates.io - License](https://img.shields.io/crates/l/sync_cow/0.0.1)

[![GitHub](https://img.shields.io/static/v1?logo=GitHub&label=&message=%20&color=grey)](https://github.com/w0xel/sync_cow)
[![open issues](https://img.shields.io/github/issues-raw/w0xel/sync_cow)](https://github.com/w0xel/sync_cow/issues)
[![open pull requests](https://img.shields.io/github/issues-pr-raw/w0xel/sync_cow)](https://github.com/w0xel/sync_cow/pulls)

Thread-safe clone-on-write container for fast concurrent writing and reading.

`SyncCow` is a container for concurrent writing and reading of data. It's intended to be a
faster alternative to `std::sync::RwLock`. Especially scenarios with many concurrent readers
heavily benefit from `SyncCow`. Reading is guaranteed to
be lock-less and return immediately. Writing is only blocked by other write-accesses, never by
any read-access. A `SyncCow` with only one writer and arbitrary readers will never block. 
As `SyncCow` stores two copies of it's contained value and read values are handed out as
`std::sync::Arc`, a program using SyncCow might have a higher memory-footprint compared to
`std::sync::RwLock`.

Note that readers might read outdated data when using the SyncCow,
as writing and reading concurrently is possible.
If that is indesireable consider `std::sync::RwLock`.


## Installation

Please use [cargo-edit](https://crates.io/crates/cargo-edit) to always add the latest version of this library:

```cmd
cargo add sync_cow
```

## Examples

See the following examples:
 - [Simple Read/Write](examples/simple.rs) - Simple showcase of `SyncCow` functions
 - [Thread Read/Write](examples/write_and_read_thread.rs) - Sharing a `SyncCow` between threads using `std::sync::Arc` for concurrent access

## License

Licensed under the

- MIT license
   ([LICENSE](LICENSE) or <http://opensource.org/licenses/MIT>)

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be 
licensed as above, without any additional terms or conditions.

## [Changelog](CHANGELOG.md)

## Versioning

`sync_cow` strictly follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html)

This includes the Rust version requirement specified above.  
Earlier Rust versions may be compatible, but this can change with minor or patch releases.

Which versions are affected by features and patches can be determined from the respective headings in [CHANGELOG.md](CHANGELOG.md).

---

*mooo*
