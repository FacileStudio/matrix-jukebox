use macroific::elements::GenericImpl;
use macroific::prelude::*;
use proc_macro2::{Delimiter, Group, Ident, TokenStream};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, DeriveInput, Generics, Token, Type};

use options::*;
use parsed_field::*;

use crate::derive_accessors::final_options::{FinalOptions, Naming};

mod final_options;
pub mod options;
mod parsed_field;

const ATTR_NAME: &str = "access";

pub struct DeriveAccessors {
    fields: Vec<ParsedField>,
    container_opts: ContainerOptions,
    ident: Ident,
    generics: Generics,
}

impl ToTokens for DeriveAccessors {
    fn to_tokens(&self, _: &mut TokenStream) {
        unimplemented!("Use `into_token_stream`")
    }

    fn into_token_stream(self) -> TokenStream
    where
        Self: Sized,
    {
        macro_rules! variations {
            (
                [
                    $final_opts: ident,
                    $container_opts: ident,
                    $naming: ident,
                    $opts: ident,
                    $tokens: ident,
                    $ident: ident,
                    $comments: ident,
                    $ty: ident
                ] => $([$lower: ident $upper: ident $render: ident]),+
                | [$last_lower: ident $last_upper: ident $last_render: ident]
            ) => {
                $(
                    match $final_opts::new(
                        $container_opts.$lower,
                        &$container_opts.defaults.$lower,
                        &$naming::$upper,
                        $opts.$lower,
                        $opts.all.as_ref(),
                        &$container_opts.defaults.all,
                    ) {
                        Some(opts) if !opts.skip => {
                            let a = ::syn::parse_quote!(#[must_use]);
                            render_common(&mut $tokens, &$ident, &$comments, &opts, Some(a));
                            $tokens.extend($render(&$ident, &$ty, opts));
                        },
                        _ => {},
                    }
                )+

                match $final_opts::new(
                    $container_opts.$last_lower,
                    &$container_opts.defaults.$last_lower,
                    &$naming::$last_upper,
                    $opts.$last_lower,
                    $opts.all.as_ref(),
                    &$container_opts.defaults.all,
                ) {
                    Some(opts) if !opts.skip => {
                        render_common(&mut $tokens, &$ident, &$comments, &opts, None);
                        $tokens.extend($last_render($ident, $ty, opts));
                    },
                    _ => {},
                }
            };
        }

        let Self {
            fields,
            container_opts,
            ident,
            mut generics,
        } = self;

        let mut out = {
            if !container_opts.bounds.is_empty() {
                generics
                    .make_where_clause()
                    .predicates
                    .extend(container_opts.bounds);
            }

            let header = GenericImpl::new(generics).with_target(ident);
            quote! {
                #[automatically_derived]
                #[allow(clippy::all)]
                #header
            }
        };

        out.append(Group::new(Delimiter::Brace, {
            let mut tokens = TokenStream::new();

            for field in fields {
                let ParsedField {
                    comments,
                    opts,
                    ident,
                    ty,
                } = field;

                if opts.skip {
                    continue;
                }

                variations!(
                    [FinalOptions, container_opts, Naming, opts, tokens, ident, comments, ty] =>
                    [get GET RENDER_GET],
                    [get_mut GET_MUT RENDER_GET_MUT]
                    | [set SET RENDER_SET]
                );
            }

            tokens
        }));

        out
    }
}

#[allow(clippy::needless_pass_by_value)]
fn render_common(
    tokens: &mut TokenStream,
    ident: &Ident,
    comments: &[Attribute],
    opts: &FinalOptions,
    must_use: Option<Attribute>,
) {
    let vis = &opts.vis;
    tokens.extend(quote! {
        #(#comments)*
        #[inline]
        #must_use
        #vis
    });

    if opts.const_fn {
        tokens.append(Ident::create("const"));
    }

    tokens.append(Ident::create("fn"));

    tokens.append(match (&opts.prefix, &opts.suffix) {
        (Some(p), Some(s)) => format_ident!("{}{ident}{}", p.as_prefix(), s.as_suffix()),
        (Some(p), None) => format_ident!("{}{ident}", p.as_prefix()),
        (None, Some(s)) => format_ident!("{ident}{}", s.as_suffix()),
        (None, None) => ident.clone(),
    });
}

fn arg_ref(owned: bool) -> Option<Token![&]> {
    if owned {
        None
    } else {
        Some(<Token![&]>::default())
    }
}

fn mk_where<T, P>(bounds: Punctuated<T, P>) -> Option<TokenStream>
where
    Punctuated<T, P>: ToTokens,
{
    if bounds.is_empty() {
        None
    } else {
        Some(quote!(where #bounds))
    }
}

fn resolve_ptr_ty(base: &Type, opt: Option<DerefKind>) -> &Type {
    match (opt, base) {
        (Some(_), Type::Ptr(ptr)) => &ptr.elem,
        (_, ty) => ty,
    }
}

type RenderFieldFn = fn(&Ident, &Type, FinalOptions) -> TokenStream;
type LastRenderFieldFn = fn(Ident, Type, FinalOptions) -> TokenStream;

const RENDER_GET: RenderFieldFn = |ident, ty, opts| {
    let arg_ref = arg_ref(opts.owned);

    let val_ref = if opts.cp || opts.owned || opts.as_ref {
        None
    } else if let Some(deref) = opts.ptr_deref {
        let tokens = deref.try_into_tokens().unwrap_or_else(move || quote!(&));
        Some(tokens)
    } else {
        Some(quote!(&))
    };

    let fn_return = if let Some(ty) = opts.ty {
        ty.into_token_stream()
    } else {
        let ty = resolve_ptr_ty(ty, opts.ptr_deref);
        quote!(#val_ref #ty)
    };

    let where_clause = mk_where(opts.bounds);

    let body = if opts.ptr_deref.is_some() {
        quote!(unsafe { #val_ref *self.#ident })
    } else if opts.as_ref {
        quote!(self.#ident.as_ref())
    } else {
        quote!(#val_ref self.#ident)
    };

    quote!((#arg_ref self) -> #fn_return #where_clause { #body })
};

const RENDER_GET_MUT: RenderFieldFn = |ident, ty, opts| {
    let fn_return = if let Some(ty) = opts.ty {
        ty.into_token_stream()
    } else {
        let ty = resolve_ptr_ty(ty, opts.ptr_deref);
        quote!(&mut #ty)
    };

    let where_clause = mk_where(opts.bounds);

    let body = if opts.as_ref {
        quote!(self.#ident.as_mut())
    } else if opts.ptr_deref.is_some() {
        quote!(unsafe { &mut *self.#ident })
    } else {
        quote!(&mut self.#ident)
    };

    quote!((&mut self) -> #fn_return #where_clause { #body })
};

const RENDER_SET: LastRenderFieldFn = |ident, ty, opts| {
    let arg_ref = arg_ref(opts.owned);

    let self_ref = if opts.cp || opts.owned {
        None
    } else {
        Some(quote!(&mut))
    };

    let arg_ty = if let Some(ref ty) = opts.ty {
        ty
    } else {
        resolve_ptr_ty(&ty, opts.ptr_deref)
    };

    let where_clause = mk_where(opts.bounds);

    let assignment = if opts.ptr_deref.is_some() {
        quote! { unsafe { *self.#ident = new_value; } }
    } else {
        quote! { self.#ident = new_value }
    };

    quote! {
        (#arg_ref mut self, new_value: #arg_ty) -> #self_ref Self #where_clause {
            #assignment;
            self
        }
    }
};

impl Parse for DeriveAccessors {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let DeriveInput {
            attrs,
            ident,
            generics,
            data,
            ..
        } = input.parse()?;

        let container_opts = ContainerOptions::from_iter_named(ATTR_NAME, ident.span(), attrs)?;
        let fields = data
            .extract_struct_named()?
            .into_iter()
            .map(ParsedField::try_from);

        Ok(Self {
            fields: fields.collect::<syn::Result<_>>()?,
            container_opts,
            ident,
            generics,
        })
    }
}
