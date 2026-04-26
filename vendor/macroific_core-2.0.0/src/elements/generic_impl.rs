//! `impl SomeTrait for SomeType`.

use crate::core_ext::*;
use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, TokenStreamExt};
use syn::{Generics, ImplGenerics, TypeGenerics, WhereClause};
use unspecified::Unspecified;

/// `impl<T> SomeTrait for SomeType<T>` or `impl<T> SomeType<T>`.
///
/// ```
/// # use macroific_core::{elements::*, core_ext::*};
/// # use proc_macro2::*;
/// # use syn::*;
/// # use quote::*;
///
/// // Make <T: Clone> where T: Copy generic bounds for our example
/// let mut generics = Generics::default();
/// generics.params.push(parse_quote!(T: Clone));
/// generics.make_where_clause().predicates.push(parse_quote!(T: Copy));
///
/// let our_trait_name: Path = parse_quote!{ ::foo::Trait };
/// let implemented_for = Ident::create("SomeStruct");
///
/// let g_impl = GenericImpl::new(&generics)
///   .with_trait(our_trait_name) // Can be anything implementing ToTokens
///   .with_target(&implemented_for) // Same
///   .into_token_stream()
///   .to_string();
/// assert_eq!(g_impl, "impl < T : Clone > :: foo :: Trait for SomeStruct < T > where T : Copy");
///
/// let g_impl = GenericImpl::new(generics) // You can give an owned generics argument
///   .with_target(implemented_for) // You can also omit `with_trait`
///   .into_token_stream()
///   .to_string();
///
/// assert_eq!(g_impl, "impl < T : Clone > SomeStruct < T > where T : Copy");
/// ```
#[cfg(feature = "generic-impl")]
pub struct GenericImpl<G, Tgt = Unspecified, Tr = Unspecified> {
    generics: G,
    implemented_for: Tgt,
    implemented_trait: Tr,
}

impl<G> GenericImpl<G> {
    /// Implement using the given [`Generics`] or [`&Generics`](Generics)…
    #[inline]
    pub fn new(generics: G) -> Self
    where
        G: GenericsLike,
    {
        Self {
            generics,
            implemented_trait: Unspecified,
            implemented_for: Unspecified,
        }
    }
}

impl<G, Tgt, Tr> GenericImpl<G, Tgt, Tr> {
    /// Implement the given trait
    #[inline]
    pub fn with_trait<Tr2>(self, trait_name: Tr2) -> GenericImpl<G, Tgt, Tr2> {
        GenericImpl {
            generics: self.generics,
            implemented_trait: trait_name,
            implemented_for: self.implemented_for,
        }
    }

    /// Implement for the given type.
    pub fn with_target<Tgt2>(self, target: Tgt2) -> GenericImpl<G, Tgt2, Tr> {
        GenericImpl {
            generics: self.generics,
            implemented_trait: self.implemented_trait,
            implemented_for: target,
        }
    }
}

impl<G, Tgt> ToTokens for GenericImpl<G, Tgt>
where
    G: GenericsLike,
    Tgt: ToTokens,
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            generics,
            implemented_for,
            implemented_trait: _,
        } = self;

        let (g1, g2, g3) = generics.split_for_impl();
        tokens.append(Ident::create("impl"));
        g1.to_tokens(tokens);
        implemented_for.to_tokens(tokens);
        g2.to_tokens(tokens);
        g3.to_tokens(tokens);
    }
}

impl<G, Tgt, Tr> ToTokens for GenericImpl<G, Tgt, Tr>
where
    G: GenericsLike,
    Tgt: ToTokens,
    Tr: ToTokens,
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            generics,
            implemented_for,
            implemented_trait,
        } = self;

        let (g1, g2, g3) = generics.split_for_impl();
        tokens.append(Ident::create("impl"));
        g1.to_tokens(tokens);
        implemented_trait.to_tokens(tokens);
        tokens.append(Ident::create("for"));
        implemented_for.to_tokens(tokens);
        g2.to_tokens(tokens);
        g3.to_tokens(tokens);
    }
}

/// [`Generics`] or something that acts like them.
pub trait GenericsLike {
    /// Mirror of [`Generics::split_for_impl`].
    fn split_for_impl(&self) -> (ImplGenerics, TypeGenerics, Option<&WhereClause>);
}

impl<T: GenericsLike> GenericsLike for &T {
    #[inline]
    fn split_for_impl(&self) -> (ImplGenerics, TypeGenerics, Option<&WhereClause>) {
        T::split_for_impl(*self)
    }
}

impl GenericsLike for Generics {
    fn split_for_impl(&self) -> (ImplGenerics, TypeGenerics, Option<&WhereClause>) {
        Generics::split_for_impl(self)
    }
}

mod unspecified {
    /// Equivalent to `()`, but guaranteed to not get `ToTokens` implemented.
    pub struct Unspecified;
}
