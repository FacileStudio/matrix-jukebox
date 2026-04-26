use crate::__attr_parse_prelude::*;
use syn::parse::{Parse, ParseStream};

/// A wrapper to make any [`ParseOption`] into a [`Parse`].
///
/// # Example
///
/// ```
/// # mod macroific { pub mod attr_parse { pub use macroific_attr_parse::*; } }
/// # use macroific::attr_parse::__attr_parse_prelude::*;
/// # use proc_macro2::Ident;
/// # use syn::parse::ParseStream;
/// # use quote::quote;
/// use macroific::attr_parse::{ValueSyntax, ParseWrapper};
///
/// // Implements ParseOption, but not Parse
/// struct MyOption(Ident);
/// impl ParseOption for MyOption {
///    fn from_stream(input: ParseStream) -> syn::Result<Self> {
///      ValueSyntax::from_stream(input).and_parse(input).map(Self)
///    }
/// }
///
/// let opt: MyOption = ParseWrapper::parse_stream_self(quote!(= foobar)).unwrap();
/// assert_eq!(opt.0, "foobar");
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[repr(transparent)]
pub struct ParseWrapper<T>(T);

impl<T> ParseWrapper<T> {
    #[allow(missing_docs)]
    #[inline]
    pub const fn new(inner: T) -> Self {
        Self(inner)
    }

    /// Get the inner value
    #[inline]
    pub fn inner(self) -> T {
        self.0
    }

    /// Shorthand for [`parse`](Parse::parse) followed by [`inner`](ParseWrapper::inner).
    pub fn parse_self(input: ParseStream) -> syn::Result<T>
    where
        T: ParseOption,
    {
        Ok(Self::parse(input)?.inner())
    }

    /// [`parse_self`](ParseWrapper::parse_self) that accepts a [`proc_macro2::TokenStream`].
    pub fn parse_stream_self(input: proc_macro2::TokenStream) -> syn::Result<T>
    where
        T: ParseOption,
    {
        Ok(syn::parse2::<Self>(input)?.inner())
    }
}

impl<T: ParseOption> Parse for ParseWrapper<T> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self::new(T::from_stream(input)?))
    }
}
