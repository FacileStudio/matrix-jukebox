use syn::parse::{Parse, ParseBuffer, ParseStream};
use syn::{parenthesized, Token};

/// Syntax used for providing a value
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ValueSyntax {
    /// `= contents`
    Eq,

    /// `(contents)`
    Paren,
}

impl ValueSyntax {
    /// Returns `true` if the syntax is [`Eq`](ValueSyntax::Eq).
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_attr_parse::ValueSyntax;
    /// #
    /// assert!(ValueSyntax::Eq.is_eq());
    /// assert!(!ValueSyntax::Paren.is_eq());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_eq(self) -> bool {
        matches!(self, Self::Eq)
    }

    /// Returns `true` if the syntax is [`Paren`](ValueSyntax::Paren).
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_attr_parse::ValueSyntax;
    /// #
    /// assert!(ValueSyntax::Paren.is_paren());
    /// assert!(!ValueSyntax::Eq.is_paren());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_paren(self) -> bool {
        matches!(self, Self::Paren)
    }

    /// Peek the stream **without moving the cursor** and attempt to construct self based on the
    /// next token.
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_attr_parse::ValueSyntax;
    /// # use syn::parse::{ParseStream, Parse};
    /// # use proc_macro2::TokenStream;
    /// # use quote::quote;
    /// #
    /// struct Foo {
    ///   peek: Option<ValueSyntax>,
    ///   rest: TokenStream,
    /// }
    ///
    /// impl Parse for Foo {
    ///   fn parse(input: ParseStream) -> syn::Result<Self> {
    ///     let peek = ValueSyntax::from_stream(input);
    ///     let rest = input.parse()?;
    ///
    ///     Ok(Self { peek, rest })
    ///   }
    /// }
    ///
    /// let v: Foo = syn::parse2(quote!(= 123)).unwrap();
    /// assert_eq!(v.peek, Some(ValueSyntax::Eq));
    /// assert_eq!(v.rest.to_string(), "= 123");
    ///
    /// let v: Foo = syn::parse2(quote!((456))).unwrap();
    /// assert_eq!(v.peek, Some(ValueSyntax::Paren));
    /// assert_eq!(v.rest.to_string(), "(456)");
    ///
    /// let v: Foo = syn::parse2(quote!(none)).unwrap();
    /// assert_eq!(v.peek, None);
    /// assert_eq!(v.rest.to_string(), "none");
    /// ```
    pub fn from_stream(parse: ParseStream) -> Option<Self> {
        if parse.peek(Token![=]) {
            Some(Self::Eq)
        } else if parse.peek(syn::token::Paren) {
            Some(Self::Paren)
        } else {
            None
        }
    }

    /// Parse whatever tokens need to be parsed based on the resolved syntax.
    /// Returns a `ParseBuffer` you should continue parsing if the syntax is
    /// [`Paren`](ValueSyntax::Paren).
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_attr_parse::ValueSyntax;
    /// # use syn::parse::{ParseStream, Parse};
    /// # use quote::quote;
    /// #
    /// /// `=` implementation
    /// struct Wrapper1(syn::LitStr);
    /// impl Parse for Wrapper1 {
    ///   fn parse(input: ParseStream) -> syn::Result<Self> {
    ///       let inner = ValueSyntax::Eq.parse_token(input)?;
    ///       assert!(inner.is_none());
    ///       Ok(Self(input.parse()?))
    ///   }
    /// }
    ///
    /// /// `(value)` implementation
    /// struct Wrapper2(syn::LitStr);
    /// impl Parse for Wrapper2 {
    ///   fn parse(input: ParseStream) -> syn::Result<Self> {
    ///       let inner = ValueSyntax::Paren.parse_token(input)?.expect("expected inner buffer");
    ///       Ok(Self(inner.parse()?))
    ///   }
    /// }
    ///
    /// let v: Wrapper1 = syn::parse2(quote!(= "foo")).unwrap();
    /// assert_eq!(v.0.value(), "foo");
    ///
    /// let v: Wrapper2 = syn::parse2(quote!(("bar"))).unwrap();
    /// assert_eq!(v.0.value(), "bar");
    /// ```
    pub fn parse_token(self, input: ParseStream) -> syn::Result<Option<ParseBuffer>> {
        match self {
            Self::Eq => {
                input.parse::<Token![=]>()?;
                Ok(None)
            }
            Self::Paren => {
                let content;
                parenthesized!(content in input);
                Ok(Some(content))
            }
        }
    }

    /// Parse whatever tokens need to be parsed based on the resolved syntax and
    /// then parse the referenced value as `P`.
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_attr_parse::ValueSyntax;
    /// # use syn::parse::{ParseStream, Parse};
    /// # use quote::quote;
    /// #
    /// /// `=` implementation
    /// struct Wrapper1(syn::LitStr);
    /// impl Parse for Wrapper1 {
    ///   fn parse(input: ParseStream) -> syn::Result<Self> {
    ///       Ok(Self(ValueSyntax::Eq.parse(input)?))
    ///   }
    /// }
    ///
    /// /// `(value)` implementation
    /// struct Wrapper2(syn::LitStr);
    /// impl Parse for Wrapper2 {
    ///   fn parse(input: ParseStream) -> syn::Result<Self> {
    ///       Ok(Self(ValueSyntax::Paren.parse(input)?))
    ///   }
    /// }
    ///
    /// let v: Wrapper1 = syn::parse2(quote!(= "foo")).unwrap();
    /// assert_eq!(v.0.value(), "foo");
    ///
    /// let v: Wrapper2 = syn::parse2(quote!(("bar"))).unwrap();
    /// assert_eq!(v.0.value(), "bar");
    /// ```
    pub fn parse<P: Parse>(self, input: ParseStream) -> syn::Result<P> {
        if let Some(inner) = self.parse_token(input)? {
            inner.parse()
        } else {
            input.parse()
        }
    }
}
