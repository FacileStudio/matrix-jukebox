//! Extension traits for [`syn`]

use sealed::sealed;
use syn::parse::{Parse, ParseBuffer, ParseStream};
use syn::LitBool;

use crate::{DelimitedIter, ValueSyntax};

/// [`ParseBuffer`] extensions
#[sealed]
pub trait ParseBufferExt {
    /// Parse a boolean attribute
    ///
    /// | Value             | Result |
    /// | ----------------- | ------- |
    /// | `my_attr`         | `true`  |
    /// | `my_attr(true)`   | `true`  |
    /// | `my_attr(false)`  | `false` |
    /// | `my_attr = true`  | `true`  |
    /// | `my_attr = false` | `false` |
    ///
    /// # Examples
    ///
    /// ```
    /// # use macroific_attr_parse::__attr_parse_prelude::*;
    /// # use syn::parse::{Parse, ParseStream};
    /// # use proc_macro2::TokenStream;
    /// # use quote::quote;
    /// #
    /// struct MyStruct(bool);
    /// impl Parse for MyStruct {
    ///    fn parse(input: ParseStream) -> syn::Result<Self> {
    ///      input.parse_bool_attr().map(Self)
    ///    }
    /// }
    ///
    /// fn check(expect: bool, tokens: TokenStream) {
    ///   let tokens_str = tokens.to_string();
    ///   match syn::parse2::<MyStruct>(tokens) {
    ///     Ok(MyStruct(actual)) => assert_eq!(actual, expect, "parsed value mismatch"),
    ///     Err(e) => panic!("Error parsing `{tokens_str}`: {e}")
    ///   }
    /// }
    ///
    /// check(true, TokenStream::new());
    /// check(true, quote!((true)));
    /// check(false, quote!((false)));
    /// check(true, quote!(= true));
    /// check(false, quote!(= false));
    /// ```
    fn parse_bool_attr(&self) -> syn::Result<bool>;

    /// Parse a valued attribute
    ///
    /// | Value                 | Result                |
    /// | --------------------- | --------------------- |
    /// | `my_attr(something)`  | `P::parse(something)` |
    /// | `my_attr = something` | `P::parse(something)` |
    ///
    /// The `my_attr(something)` syntax should be preferred as `my_attr = something` can't always
    /// deserialise some types (e.g. [`Visibility`](syn::Visibility)).
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_attr_parse::__attr_parse_prelude::*;
    /// # use proc_macro2::Ident;
    /// # use syn::parse_quote;
    /// # use syn::parse::{Parse, ParseStream};
    /// #
    /// struct MyStruct(Ident);
    /// impl Parse for MyStruct {
    ///    fn parse(input: ParseStream) -> syn::Result<Self> {
    ///      input.parse_valued_attr().map(Self)
    ///    }
    /// }
    ///
    /// let v: MyStruct = parse_quote!((foo));
    /// assert_eq!(v.0, "foo");
    ///
    /// let v: MyStruct = parse_quote!(= bar);
    /// assert_eq!(v.0, "bar");
    /// ```
    fn parse_valued_attr<P: Parse>(&self) -> syn::Result<P>;

    /// Shortcut for [`DelimitedIter::new`]
    fn iter_delimited<T, D>(&self) -> DelimitedIter<T, D>
    where
        T: Parse,
        D: Parse;
}

#[sealed]
impl ParseBufferExt for ParseBuffer<'_> {
    fn parse_bool_attr(&self) -> syn::Result<bool> {
        Ok(if let Some(syntax) = ValueSyntax::from_stream(self) {
            syntax.parse::<LitBool>(self)?.value
        } else {
            true
        })
    }

    fn parse_valued_attr<P: Parse>(&self) -> syn::Result<P> {
        ValueSyntax::from_stream(self).and_parse(self)
    }

    fn iter_delimited<T, D>(&self) -> DelimitedIter<T, D>
    where
        T: Parse,
        D: Parse,
    {
        DelimitedIter::new(self)
    }
}

/// [`Option`] extensions
#[sealed]
pub trait OptionExt {
    /// If the `Option<ValueSyntax>` is `Some`, parse it based on the [`ValueSyntax`],
    /// otherwise just parse it.
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_attr_parse::__attr_parse_prelude::*;
    /// # use macroific_attr_parse::ValueSyntax;
    /// # use syn::parse_quote;
    /// # use syn::parse::{Parse, ParseStream};
    /// # use proc_macro2::Ident;
    /// #
    /// struct MyStruct(Ident);
    /// impl Parse for MyStruct {
    ///   fn parse(input: ParseStream) -> syn::Result<Self> {
    ///     ValueSyntax::from_stream(input).and_parse(input).map(Self)
    ///   }
    /// }
    ///
    /// let v: MyStruct = parse_quote!(foo);
    /// assert_eq!(v.0, "foo");
    ///
    /// let v: MyStruct = parse_quote!(= bar);
    /// assert_eq!(v.0, "bar");
    ///
    /// let v: MyStruct = parse_quote!((qux));
    /// assert_eq!(v.0, "qux");
    /// ```
    fn and_parse<P: Parse>(self, input: ParseStream) -> syn::Result<P>;
}

#[sealed]
impl OptionExt for Option<ValueSyntax> {
    fn and_parse<P: Parse>(self, input: ParseStream) -> syn::Result<P> {
        if let Some(syntax) = self {
            syntax.parse(input)
        } else {
            input.parse()
        }
    }
}
