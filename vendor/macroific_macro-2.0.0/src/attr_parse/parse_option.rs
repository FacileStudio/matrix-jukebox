use proc_macro2::Span;
use syn::{Attribute, DeriveInput, Token};

use macroific_attr_parse::AttributeOptions;
use macroific_attr_parse::__private::decode_attr_options_field;
use macroific_core::core_ext::MacroificCoreIdentExt;
use macroific_core::elements::{GenericImpl, ModulePrefix};

use super::{
    Delimiter, Fields, Generics, Group, Ident, ParseStream, Render, ToTokens, TokenStream, RESULT,
};
use super::{ATTR_NAME, BASE, PRIVATE};
use ::syn::parse::Parse;
use quote::{quote, TokenStreamExt};

struct Options {
    from_parse: bool,
}

impl AttributeOptions for Options {
    fn from_iter(_: Span, attributes: impl IntoIterator<Item = Attribute>) -> syn::Result<Self> {
        let mut from_parse = None;

        for attr in attributes {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("from_parse") {
                    decode_attr_options_field(&mut from_parse, &meta.path, meta.input)
                } else {
                    Ok(())
                }
            })?;
        }

        Ok(Self {
            from_parse: from_parse.unwrap_or(false),
        })
    }
}

impl Render for ParseOptionDerive {
    const TRAIT_NAME: &'static str = "ParseOption";

    #[inline]
    fn generics(&self) -> &Generics {
        &self.as_ref().generics
    }

    #[inline]
    fn ident(&self) -> &Ident {
        &self.as_ref().ident
    }

    fn fields(&self) -> &Fields {
        match self {
            Self::Base(_, fields) => fields,
            Self::FromParse(_) => {
                unreachable!("`fields` inapplicable for `FromParse`")
            }
        }
    }

    fn render_empty_body(ending: Option<Group>) -> TokenStream {
        quote! {
            #[inline]
            fn from_stream(_: ::syn::parse::ParseStream) -> ::syn::Result<Self> {
                #RESULT::Ok(Self #ending)
            }
        }
    }
}

impl AsRef<ParseOptionCommonData> for ParseOptionDerive {
    fn as_ref(&self) -> &ParseOptionCommonData {
        match self {
            Self::Base(c, _) | Self::FromParse(c) => c,
        }
    }
}

pub enum ParseOptionDerive {
    Base(ParseOptionCommonData, Fields),
    FromParse(ParseOptionCommonData),
}

pub struct ParseOptionCommonData {
    ident: Ident,
    generics: Generics,
}

impl Parse for ParseOptionDerive {
    fn parse(input: ParseStream) -> ::syn::Result<Self> {
        let DeriveInput {
            ident,
            generics,
            data,
            attrs,
            ..
        } = input.parse()?;

        let opts = Options::from_iter_named(ATTR_NAME, Span::call_site(), attrs)?;
        let common = ParseOptionCommonData { ident, generics };

        Ok(if opts.from_parse {
            Self::FromParse(common)
        } else {
            Self::Base(common, data.try_into()?)
        })
    }
}

impl ParseOptionDerive {
    #[inline]
    fn to_tokens_from_parse(&self) -> TokenStream {
        let mut tokens = self.impl_generics();

        // Impl body
        tokens.append(Group::new(
            Delimiter::Brace,
            quote! {
                #[inline]
                fn from_stream(stream: ::syn::parse::ParseStream) -> ::syn::Result<Self> {
                    #PRIVATE::decode_parse_option_from_parse(stream)
                }
            },
        ));

        tokens
    }

    #[inline]
    fn to_tokens_base(&self) -> TokenStream {
        let fields = match self.named_fields() {
            Ok(fields) => fields,
            Err(delim) => return self.render_empty(delim),
        };

        let mut tokens = self.impl_generics();

        let fn_body = Group::new(Delimiter::Brace, {
            let indexed_fields = super::indexed_fields(fields);

            let matches = indexed_fields.clone()
                .map(move |(option_var_name, field)| {
                    let mut stream = field.resolved_label().into_token_stream();
                    <Token![=>]>::default().to_tokens(&mut stream);

                    stream.append(Group::new(
                        Delimiter::Brace,
                        quote! { #PRIVATE::decode_parse_option_field(&mut #option_var_name, ident, value_source) },
                    ));

                    stream
                });

            let mut out = super::nones(fields);

            out.append_all(quote! {
                // Provided ident, but no value, then continued to provide the next ident
                if !parse.peek(::syn::Token![,]) {
                    for result in #PRIVATE::iterate_option_meta(parse)? {
                        let (ident, value_source) = result?;

                        match ::std::string::ToString::to_string(&ident).as_str() {
                            #(#matches)*
                            other => return #RESULT::Err(::syn::Error::new(::syn::spanned::Spanned::span(&ident), ::std::format!("Unrecognised attribute: `{}`", other))),
                        }?;

                    }
                }
            });

            RESULT.to_tokens(&mut out);
            <Token![::]>::default().to_tokens(&mut out);
            out.append(Ident::create("Ok"));

            out.append(Group::new(Delimiter::Parenthesis, {
                let mut out = <Token![Self]>::default().into_token_stream();
                let unwraps = super::unwraps(
                    indexed_fields,
                    &quote! {
                        ::proc_macro2::Span::call_site()
                    },
                );
                out.append(unwraps);
                out
            }));

            out
        });

        // Impl body
        tokens.append(Group::new(Delimiter::Brace, {
            let mut signature = quote! {
                fn from_stream(parse: ::syn::parse::ParseStream) -> ::syn::Result<Self>
            };
            signature.append(fn_body);
            signature
        }));

        GenericImpl::new(self.generics())
            .with_trait(ModulePrefix::new(["syn", "parse", "Parse"]))
            .with_target(self.ident())
            .to_tokens(&mut tokens);

        tokens.append(Group::new(
            Delimiter::Brace,
            quote! {
                #[inline]
                fn parse(parse: ::syn::parse::ParseStream) -> ::syn::Result<Self> {
                    #BASE::ParseOption::from_stream(parse)
                }
            },
        ));

        tokens
    }
}

impl ToTokens for ParseOptionDerive {
    fn to_tokens(&self, _: &mut TokenStream) {
        unimplemented!("Use to_token_stream")
    }

    fn to_token_stream(&self) -> TokenStream {
        match self {
            Self::FromParse(_) => self.to_tokens_from_parse(),
            Self::Base(_, _) => self.to_tokens_base(),
        }
    }
}
