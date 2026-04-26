use std::fmt::Write;

use proc_macro2::TokenStream;
use proc_macro2::{Delimiter, Group, Ident, Punct};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Generics};

pub use attr_options::AttrOptionsDerive;
use macroific_core::core_ext::{MacroificCoreIdentExt, MacroificCorePunctExt};
use macroific_core::elements::module_prefix::{OPTION, RESULT};
use macroific_core::elements::{GenericImpl, ModulePrefix};
use options::*;
pub use parse_option::ParseOptionDerive;

use crate::BaseTokenStream;

mod attr_options;
mod options;
mod parse_option;

const ATTR_NAME: &str = "attr_opts";

const PRIVATE: ModulePrefix<'static, 3> =
    ModulePrefix::new(["macroific", "attr_parse", "__private"]);
const BASE: ModulePrefix<'static, 2> = ModulePrefix::new(["macroific", "attr_parse"]);

trait Render {
    const TRAIT_NAME: &'static str;

    fn generics(&self) -> &Generics;
    fn ident(&self) -> &Ident;
    fn fields(&self) -> &Fields;

    fn render_empty_body(ending: Option<Group>) -> TokenStream;

    #[inline]
    fn impl_generics(&self) -> TokenStream {
        impl_generics(self.generics(), self.ident(), Self::TRAIT_NAME)
    }

    fn named_fields(&self) -> Result<&[Field], Option<Delimiter>> {
        match *self.fields() {
            Fields::Named(ref fields) => Ok(fields),
            Fields::Empty(delim) => Err(Some(delim)),
            Fields::Unit => Err(None),
        }
    }

    fn render_empty(&self, delimiter: Option<Delimiter>) -> TokenStream {
        let ending = empty_ending(delimiter);
        let mut tokens = self.impl_generics();

        tokens.append(Group::new(
            Delimiter::Brace,
            Self::render_empty_body(ending),
        ));

        tokens
    }
}

fn nones(fields: &[Field]) -> TokenStream {
    (0..fields.len())
        .map(move |idx| {
            let ident = field_ident_at(idx);
            quote! { let mut #ident = #OPTION::None; }
        })
        .collect()
}

fn unwraps<'a>(
    indexed_fields: impl Iterator<Item = IndexedFieldTuple<'a>>,
    span_arg_name: &impl ToTokens,
) -> Group {
    let body = indexed_fields.map(move |(option_var_name, field)| {
        let mut out = field.ident.to_token_stream();
        out.append(Punct::new_joint(':'));

        match field.opts.default {
            None | Some(DefaultOption::Implicit | DefaultOption::Explicit(true)) => {
                out.extend(quote! { #option_var_name.unwrap_or_default() });
            }
            Some(DefaultOption::Explicit(false)) => {
                let mut missing_field_err = String::from("Missing required attribute: ");
                if let Some(ref rename) = field.opts.rename {
                    write!(&mut missing_field_err, "{}", rename.token()).unwrap();
                } else {
                    write!(&mut missing_field_err, "{}", field.ident).unwrap();
                }

                out.extend(quote! { if let #OPTION::Some(v) = #option_var_name {
                    v
                } else {
                    return #RESULT::Err(::syn::Error::new(#span_arg_name, #missing_field_err));
                } });
            }
            Some(DefaultOption::Path(ref path)) => {
                out.extend(quote! { #option_var_name.unwrap_or_else(#path) });
            }
        };

        out.append(Punct::new_joint(','));

        out
    });

    Group::new(Delimiter::Brace, body.collect())
}

type IndexedFieldTuple<'a> = (Ident, &'a Field);

fn indexed_fields(fields: &[Field]) -> impl Iterator<Item = IndexedFieldTuple> + Clone {
    fields.iter().enumerate().map(move |(idx, field)| {
        let option_var_name = field_ident_at(idx);

        (option_var_name, field)
    })
}

fn empty_ending(delimiter: Option<Delimiter>) -> Option<Group> {
    delimiter.map(move |d| Group::new(d, TokenStream::new()))
}

pub fn run<T: Parse + ToTokens>(input: BaseTokenStream) -> BaseTokenStream {
    parse_macro_input!(input as T).into_token_stream().into()
}

fn impl_generics(generics: &Generics, ident: &Ident, trait_name: &str) -> TokenStream {
    let impl_trait = {
        let trait_name = Ident::create(trait_name);
        quote!(#BASE::#trait_name)
    };
    let mut tokens = quote!(#[automatically_derived]);

    GenericImpl::new(generics)
        .with_trait(impl_trait)
        .with_target(ident)
        .to_tokens(&mut tokens);

    tokens
}

fn field_ident_at(idx: usize) -> Ident {
    format_ident!("field{idx}")
}
