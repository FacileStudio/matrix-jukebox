use super::opts::MultiContainerOptions;
use super::Implementation;
use crate::ATTR_FMT;
use macroific::prelude::*;
use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Span, TokenStream};
use syn::{DeriveInput, Error};

#[derive(Copy, Clone)]
pub struct Alias<'a> {
    pub attr_name: &'a str,
    pub trait_name: &'a str,
}

impl<'a> Implementation<'a> {
    pub fn exec_compound(input: TokenStream1) -> TokenStream1 {
        Self::exec_compound_2(input)
            .unwrap_or_else(Error::into_compile_error)
            .into()
    }

    fn exec_compound_2(input: TokenStream1) -> syn::Result<TokenStream> {
        let DeriveInput {
            attrs,
            ident,
            generics,
            data,
            ..
        } = syn::parse(input)?;

        let tokens = MultiContainerOptions::from_iter_named(ATTR_FMT, Span::call_site(), attrs)?
            .into_iter()
            .map(move |(alias, opts)| -> syn::Result<TokenStream> {
                let for_alias = Self {
                    opts,
                    trait_name: alias.trait_name,
                    ident: ident.clone(),
                    generics: generics.clone(),
                };

                for_alias.exec_data(data.clone(), alias.attr_name)
            })
            .collect::<syn::Result<TokenStream>>()?;

        if tokens.is_empty() {
            Err(Error::call_site(format!(
                "Missing #[{ATTR_FMT}] attribute."
            )))
        } else {
            Ok(tokens)
        }
    }
}
