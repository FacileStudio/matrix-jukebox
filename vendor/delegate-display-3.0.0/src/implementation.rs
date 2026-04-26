mod compound;
mod dual_attr;
mod main_field;
mod opts;
mod variant;

pub use compound::Alias;

use main_field::MainField;
use variant::{Style, Variant};

use macroific::elements::module_prefix::RESULT;
use macroific::elements::{GenericImpl, ModulePrefix};
use macroific::prelude::*;
use opts::ContainerOptions;
use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{parse_quote, Data, DeriveInput, Error, Generics};

const FMT: ModulePrefix<2> = ModulePrefix::new(["core", "fmt"]);

pub(crate) struct Implementation<'a> {
    trait_name: &'a str,
    ident: Ident,
    generics: Generics,
    opts: ContainerOptions,
}

impl<'a> Implementation<'a> {
    pub fn exec(input: TokenStream1, attr_name: &str, trait_name: &'a str) -> TokenStream1 {
        Self::exec_2(input, attr_name, trait_name)
            .unwrap_or_else(Error::into_compile_error)
            .into()
    }

    fn exec_2(
        input: TokenStream1,
        attr_name: &str,
        trait_name: &'a str,
    ) -> syn::Result<TokenStream> {
        let DeriveInput {
            attrs,
            ident,
            generics,
            data,
            ..
        } = syn::parse(input)?;

        let common = Self {
            opts: ContainerOptions::resolve(attrs, attr_name)?,
            trait_name,
            ident,
            generics,
        };

        common.exec_data(data, attr_name)
    }

    fn exec_data(self, data: Data, attr_name: &str) -> syn::Result<TokenStream> {
        match data {
            Data::Struct(data) => {
                let main_field = MainField::resolve_from_fields(data.fields, attr_name)?;
                Ok(self.impl_struct(main_field))
            }
            Data::Enum(data) => {
                let variants: Vec<Variant> = data
                    .variants
                    .into_iter()
                    .map(move |v| Variant::from_syn(v, attr_name))
                    .collect::<syn::Result<_>>()?;

                Ok(self.impl_enum(variants))
            }
            Data::Union(u) => Err(Error::new_spanned(u.union_token, "Unions not supported")),
        }
    }

    fn trait_path(&self) -> ModulePrefix<3> {
        ModulePrefix::new(["core", "fmt", self.trait_name])
    }

    fn impl_enum(mut self, variants: Vec<Variant>) -> TokenStream {
        self.preprocess_generics_enum(&variants);
        let mut tokens = self.header();
        let mut has_skipped_arms = false;
        let trait_path = self.trait_path();

        let arms = variants.into_iter()
            .filter_map(|variant| {
                let Variant { ident, style, main_field } = variant;

                match style {
                    Style::Tuple => {
                        let Some(main_field) = main_field else {
                            has_skipped_arms = true;
                            return None;
                        };

                        let args = main_field.args_for_tuple_enum();
                        let ty = if let Some(delegate_to) = &self.opts.delegate_to {
                            delegate_to
                        } else {
                            &main_field.ty
                        };

                        Some(quote! {
                            Self::#ident(#(#args),*) => <#ty as #trait_path>::fmt(v, f),
                        })
                    },
                    Style::Named => {
                        let Some(main_field) = main_field else {
                            has_skipped_arms = true;
                            return None;
                        };

                        let dots = if main_field.num_fields > 1 {
                            quote!(,..)
                        } else {
                            TokenStream::new()
                        };

                        let field_name = &main_field.ident;
                        let ty = if let Some(delegate_to) = &self.opts.delegate_to {
                            delegate_to
                        } else {
                            &main_field.ty
                        };

                        Some(quote! {
                            Self::#ident { #field_name: v #dots } => <#ty as #trait_path>::fmt(v, f),
                        })
                    },
                    Style::Unit => {
                        has_skipped_arms = true;
                        None
                    },
                }
            })
            .collect::<TokenStream>();

        let other_arm = if has_skipped_arms {
            quote!(_ => #RESULT::Ok(()),)
        } else {
            TokenStream::new()
        };

        let formatter_name = Ident::create(if arms.is_empty() { "_" } else { "f" });

        tokens.extend(quote! {{
            fn fmt(&self, #formatter_name: &mut #FMT::Formatter<'_>) -> #FMT::Result {
                match self {
                    #arms
                    #other_arm
                }
            }
        }});

        tokens
    }

    fn impl_struct(mut self, main_field: Option<MainField>) -> TokenStream {
        self.preprocess_generics_struct(&main_field);
        let mut tokens = self.header();
        let trait_path = self.trait_path();

        let (body, param) = if let Some(main_field) = main_field {
            let ident = main_field.ident_for_struct();
            let ty = if let Some(delegate_to) = &self.opts.delegate_to {
                delegate_to
            } else {
                &main_field.ty
            };

            (
                quote!(<#ty as #trait_path>::fmt(&self.#ident, f)),
                Ident::create("f"),
            )
        } else {
            (quote!(#RESULT::Ok(())), Ident::create("_"))
        };

        tokens.extend(quote! {{
            #[inline]
            fn fmt(&self, #param: &mut #FMT::Formatter<'_>) -> #FMT::Result {
                #body
            }
        }});

        tokens
    }

    fn header(&self) -> TokenStream {
        let header = GenericImpl::new(&self.generics)
            .with_trait(self.trait_path())
            .with_target(&self.ident);

        quote! {
            #[automatically_derived]
            #[allow(clippy::all)]
            #header
        }
    }

    fn preprocess_generics_struct(&mut self, main_field: &Option<MainField>) {
        if self.generics.params.is_empty() {
            return;
        }

        if self.preprocess_generics_common() {
            return;
        }

        if let Some(main_field) = main_field {
            self.add_debug_clause(&main_field.ty);
        }
    }

    fn preprocess_generics_enum(&mut self, variants: &[Variant]) {
        if self.generics.params.is_empty() {
            return;
        }

        if self.preprocess_generics_common() {
            return;
        }

        for variant in variants {
            if let Some(main_field) = &variant.main_field {
                self.add_debug_clause(&main_field.ty);
            }
        }
    }

    fn preprocess_generics_common(&mut self) -> bool {
        if !self.opts.bounds.is_empty() {
            let iter = self.opts.bounds.iter().cloned();
            self.generics.make_where_clause().predicates.extend(iter);
            true
        } else if let Some(delegate_to) = &self.opts.delegate_to {
            self.add_debug_clause(delegate_to.clone());
            true
        } else {
            false
        }
    }

    fn add_debug_clause<T>(&mut self, ty: T)
    where
        T: ToTokens,
    {
        let predicate = {
            let path = self.trait_path();
            parse_quote!(#ty: #path)
        };

        self.generics.make_where_clause().predicates.push(predicate);
    }
}
