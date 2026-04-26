use super::main_field::MainField;
use proc_macro2::Ident;
use syn::spanned::Spanned;

#[derive(Copy, Clone)]
pub(crate) enum Style {
    Named,
    Tuple,
    Unit,
}

pub(crate) struct Variant {
    pub ident: Ident,
    pub style: Style,
    pub main_field: Option<MainField>,
}

impl Variant {
    pub fn from_syn(variant: syn::Variant, attr_name: &str) -> syn::Result<Self> {
        let (style, main_field) = match variant.fields {
            syn::Fields::Named(f) => {
                let span = f.span();
                (
                    Style::Named,
                    MainField::resolve_from_iter(f.named, attr_name, span)?,
                )
            }
            syn::Fields::Unnamed(f) => {
                let span = f.span();
                (
                    Style::Tuple,
                    MainField::resolve_from_iter(f.unnamed, attr_name, span)?,
                )
            }
            syn::Fields::Unit => (Style::Unit, None),
        };

        Ok(Self {
            ident: variant.ident,
            style,
            main_field,
        })
    }
}
