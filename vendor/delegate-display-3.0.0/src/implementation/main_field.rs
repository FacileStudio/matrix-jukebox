use super::dual_attr::AttrKind;
use crate::ATTR_ANY;
use macroific::prelude::*;
use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use std::fmt::Display;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Error, LitInt, Type};

pub(crate) struct MainField {
    pub idx: usize,
    pub ident: Option<Ident>,
    pub ty: Type,
    pub num_fields: usize,
    pub mark: Option<AttrKind>,
}

impl MainField {
    pub fn ident_for_struct(&self) -> TokenStream {
        if let Some(ident) = &self.ident {
            ident.to_token_stream()
        } else {
            LitInt::new(itoa::Buffer::new().format(self.idx), Span::call_site()).into_token_stream()
        }
    }

    pub fn args_for_tuple_enum(&self) -> impl Iterator<Item = Ident> {
        fn empty_ident<T>(_: T) -> Ident {
            Ident::create("_")
        }

        let pre = (0..self.idx).map(empty_ident);
        let post = ((self.idx + 1)..self.num_fields).map(empty_ident);

        pre.chain(Some(Ident::create("v"))).chain(post)
    }

    pub fn resolve_from_fields(fields: syn::Fields, attr_name: &str) -> syn::Result<Option<Self>> {
        match fields {
            syn::Fields::Named(f) => {
                let span = f.span();
                Self::resolve_from_iter(f.named, attr_name, span)
            }
            syn::Fields::Unnamed(f) => {
                let span = f.span();
                Self::resolve_from_iter(f.unnamed, attr_name, span)
            }
            syn::Fields::Unit => Ok(None),
        }
    }

    pub fn resolve_from_iter<P>(
        fields: Punctuated<syn::Field, P>,
        attr_name: &str,
        span: Span,
    ) -> syn::Result<Option<Self>> {
        let num_fields = fields.len();
        let mut fields = fields.into_iter().enumerate();

        let mut first_field = match fields.next() {
            Some((idx, field)) => Self {
                num_fields,
                mark: AttrKind::aggregate(field.attrs, attr_name),
                idx,
                ident: field.ident,
                ty: field.ty,
            },
            None => return Ok(None),
        };

        for (idx, field) in fields {
            let span = field.span();

            match AttrKind::aggregate(field.attrs, attr_name) {
                None => {}
                Some(AttrKind::Primary) => match first_field.mark {
                    None | Some(AttrKind::CatchAll) => {
                        first_field.update(AttrKind::Primary, idx, field.ident, field.ty);
                    }
                    Some(AttrKind::Primary) => return Err(duplicate_err(attr_name, span)),
                },
                Some(AttrKind::CatchAll) => match first_field.mark {
                    None => {
                        first_field.update(AttrKind::CatchAll, idx, field.ident, field.ty);
                    }
                    Some(AttrKind::Primary) => {}
                    Some(AttrKind::CatchAll) => return Err(duplicate_err(ATTR_ANY, span)),
                },
            }
        }

        if num_fields > 1 && first_field.mark.is_none() {
            let msg = format!("At least one field must be marked with #[{attr_name}] or #[{ATTR_ANY}] on types with more than one field");
            Err(Error::new(span, msg))
        } else {
            Ok(Some(first_field))
        }
    }

    fn update(&mut self, mark: AttrKind, idx: usize, ident: Option<Ident>, ty: Type) {
        self.mark = Some(mark);
        self.idx = idx;
        self.ident = ident;
        self.ty = ty;
    }
}

fn duplicate_err<T>(attr_name: T, span: Span) -> Error
where
    T: Display,
{
    let msg = format!("Multiple fields marked with #[{attr_name}]");
    Error::new(span, msg)
}
