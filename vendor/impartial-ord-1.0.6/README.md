<!-- cargo-rdme start -->

Derives a quicker `PartialOrd` for types that already implement `Ord`.

[![Master CI badge](https://github.com/Alorel/impartial-ord-rs/actions/workflows/lint-n-test.yml/badge.svg?branch=master)](https://github.com/Alorel/impartial-ord-rs/actions/workflows/lint-n-test.yml?query=branch%3Amaster)
[![crates.io badge](https://img.shields.io/crates/v/impartial-ord)](https://crates.io/crates/impartial-ord)
[![docs.rs badge](https://img.shields.io/docsrs/impartial-ord?label=docs.rs)](https://docs.rs/impartial-ord)
[![dependencies badge](https://img.shields.io/librariesio/release/cargo/impartial-ord)](https://libraries.io/cargo/impartial-ord)

```rust
#[derive(impartial_ord::ImpartialOrd, Ord, PartialEq, Eq, Default, Debug)]
struct MyStruct { foo: Bar, qux: Baz, }

assert_eq!(MyStruct::default().partial_cmp(&MyStruct::default()), Some(Ordering::Equal));
```

### Generated output

```rust
impl PartialOrd for MyStruct where Self: Ord {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Ord::cmp(self, other))
    }
}
````

<!-- cargo-rdme end -->
