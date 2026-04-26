<!-- cargo-rdme start -->

Proc macro development utilities

[![MASTER CI status](https://github.com/Alorel/macroific-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/Alorel/macroific-rs/actions/workflows/ci.yml?query=branch%3Amaster)
[![crates.io badge](https://img.shields.io/crates/v/macroific)](https://crates.io/crates/macroific)
[![Coverage Status](https://coveralls.io/repos/github/Alorel/macroific-rs/badge.svg)](https://coveralls.io/github/Alorel/macroific-rs)
[![dependencies badge](https://img.shields.io/librariesio/release/cargo/macroific)](https://libraries.io/cargo/macroific)

# Features

| Feature | Description |
| ------- | ----------- |
| `attr_parse` | Attribute parsing utilities, [`attr_parse`](https://docs.rs/macroific/latest/macroific/attr_parse/). |
| `full` | Enable `syn/full`. If `attr_parse` is enabled, it'll implement the traits for types that require `syn/full`. |
| `attributed` | Enable [`elements::Attributed`]. |
| `generic-impl` | Enable [`elements::GenericImpl`]. |
| `module-prefix` | Enable [`elements::ModulePrefix`]. |

<!-- cargo-rdme end -->
