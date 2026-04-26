//! Derives a quicker `PartialOrd` for types that already implement `Ord`.
//!
//! [![Master CI badge](https://github.com/Alorel/impartial-ord-rs/actions/workflows/lint-n-test.yml/badge.svg?branch=master)](https://github.com/Alorel/impartial-ord-rs/actions/workflows/lint-n-test.yml?query=branch%3Amaster)
//! [![crates.io badge](https://img.shields.io/crates/v/impartial-ord)](https://crates.io/crates/impartial-ord)
//! [![docs.rs badge](https://img.shields.io/docsrs/impartial-ord?label=docs.rs)](https://docs.rs/impartial-ord)
//! [![dependencies badge](https://img.shields.io/librariesio/release/cargo/impartial-ord)](https://libraries.io/cargo/impartial-ord)
//!
//! ```
//! # use core::cmp::{PartialOrd, Ord, Ordering};
//! # type Bar = u8;
//! # type Baz = u8;
//! #
//! #[derive(impartial_ord::ImpartialOrd, Ord, PartialEq, Eq, Default, Debug)]
//! struct MyStruct { foo: Bar, qux: Baz, }
//!
//! assert_eq!(MyStruct::default().partial_cmp(&MyStruct::default()), Some(Ordering::Equal));
//! ```
//!
//! ### Generated output
//!
#![cfg_attr(doctest, doc = " ````no_test")]
//! ```
//! impl PartialOrd for MyStruct where Self: Ord {
//!     #[inline]
//!     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//!         Some(Ord::cmp(self, other))
//!     }
//! }
//! ````
//!

use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput};

/// A quicker [PartialOrd](core::cmp::PartialOrd) for types that already implement
/// [Ord](core::cmp::Ord). See [crate-level docs](crate).
#[proc_macro_derive(ImpartialOrd)]
pub fn derive_partial_ord(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let DeriveInput {
        ident,
        mut generics,
        ..
    } = parse_macro_input!(input as DeriveInput);

    let (gen_impl, gen_type, gen_where) = {
        generics
            .make_where_clause()
            .predicates
            .push(parse_quote! { Self: ::core::cmp::Ord });
        generics.split_for_impl()
    };

    (quote! {
        #[automatically_derived]
        impl #gen_impl ::core::cmp::PartialOrd for #ident #gen_type #gen_where {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::option::Option::Some(::core::cmp::Ord::cmp(self, other))
            }
        }
    })
    .into()
}
