# ars

[![License](https://img.shields.io/github/license/ryuapp/ars?labelColor=171717&color=39b54a&label=License)](https://github.com/ryuapp/ars/blob/main/LICENSE)
[![npm](https://img.shields.io/crates/v/ars?labelColor=171717&color=39b54a)](https://crates.io/crates/ars)
[![github repo](https://img.shields.io/badge/GitHub-ryuapp/ars-171717?labelColor=171717&color=39b54a)](https://github.com/ryuapp/ars)

A pure Rust implementation of URL library based on [ada-url](https://github.com/ada-url/ada).\
This library is experimental; for production use, consider using [url](https://github.com/servo/rust-url) crate or [ada-url/rust](https://github.com/ada-url/rust).

## Why not use `url`?

[url](https://github.com/servo/rust-url) is a widely used Rust URL parsing library. However, this library does not pass 100% of Web Platform Tests. Additionally, creating JavaScript bindings for this Rust-native crate requires some adaptation. ars is designed to address these limitations.

## Why not use `ada-url`?

[ada-url](https://github.com/ada-url/ada) is a fast and WHATWG-compliant URL parsing library written in C++. This library is used by many projects, including Node.js, and provides [ada-url/rust](https://github.com/ada-url/rust) as Rust bindings. Unlike the `url` crate, `ada-url` passes 100% of WPT. However, using C++ in a Rust library complicates the build environment for some targets. ars is written entirely in Rust to avoid this complexity.

## License

MIT

This project's tests and benchmarking code incorporate material from third parties, but they are provided for research purposes only and not part of the library.
