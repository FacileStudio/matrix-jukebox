use proc_macro2::TokenStream;
use quote::ToTokens;
use std::marker::PhantomData;
use syn::parse::{Parse, ParseStream};
use syn::Attribute;

/// [`Attributed`] alias that uses [`Attribute::parse_inner`] for parsing.
#[cfg(feature = "attributed")]
pub type AttributedInner<T = TokenStream> = Attributed<T, kind::Inner>;

/// Something that has [`Attribute`]s. Use when you case about processing that has attributes such
/// as doc comments, but aren't picky about what they're attached to.
///
/// See examples on [`Attributed::parse_outer`] and [`AttributedInner::parse_inner`].
#[cfg(feature = "attributed")]
pub struct Attributed<T = TokenStream, K = kind::Outer> {
    /// Collected attributes.
    pub attributes: Vec<Attribute>,

    /// The data following the attributes.
    pub data: T,

    _kind: PhantomData<K>,
}

impl<T: Parse> Attributed<T> {
    /// Parse using [`Attribute::parse_outer`].
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_core::elements::Attributed;
    /// # use syn::parse_quote;
    /// # use quote::ToTokens;
    /// #
    /// let attr: Attributed = parse_quote!(#[foo]);
    /// assert_eq!(attr.into_token_stream().to_string(), "# [foo]");
    /// ```
    #[allow(clippy::missing_errors_doc)]
    pub fn parse_outer(input: ParseStream) -> syn::Result<Self> {
        Ok(Self::new(Attribute::parse_outer(input)?, input.parse()?))
    }
}

impl<T: Parse> AttributedInner<T> {
    /// Parse using [`Attribute::parse_inner`].
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_core::elements::AttributedInner;
    /// # use syn::parse_quote;
    /// # use quote::ToTokens;
    /// #
    /// let attr: AttributedInner = parse_quote!(#![bar]);
    /// assert_eq!(attr.into_token_stream().to_string(), "# ! [bar]");
    /// ```
    #[allow(clippy::missing_errors_doc)]
    pub fn parse_inner(input: ParseStream) -> syn::Result<Self> {
        Ok(Self::new(Attribute::parse_inner(input)?, input.parse()?))
    }
}

impl<T, K> Attributed<T, K> {
    #[inline]
    fn new(attributes: Vec<Attribute>, data: T) -> Self {
        Self {
            attributes,
            data,
            _kind: PhantomData,
        }
    }
}

impl<T: ToTokens, K> ToTokens for Attributed<T, K> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for attr in &self.attributes {
            attr.to_tokens(tokens);
        }

        self.data.to_tokens(tokens);
    }
}

impl<T: Parse> Parse for Attributed<T> {
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Self::parse_outer(input)
    }
}

impl<T: Parse> Parse for AttributedInner<T> {
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Self::parse_inner(input)
    }
}

#[allow(missing_docs)]
mod kind {
    /// Marker for making [`Attributed`](super::Attributed) parse using
    /// [`Attribute::parse_outer`](syn::Attribute::parse_outer).
    pub struct Outer;

    /// Marker for making [`Attributed`](super::Attributed) parse using
    /// [`Attribute::parse_inner`](syn::Attribute::parse_inner).
    pub struct Inner;
}
