use super::compound::Alias;
use super::dual_attr::DualAttr;
use macroific::prelude::*;
use std::iter::FusedIterator;
use std::ops::{Add, AddAssign};
use syn::punctuated::Punctuated;
use syn::{Attribute, Token, Type, WherePredicate};

#[derive(AttributeOptions, ParseOption, Default, Clone)]
pub(crate) struct ContainerOptions {
    pub bounds: Punctuated<WherePredicate, Token![,]>,
    pub delegate_to: Option<Type>,
}

#[derive(AttributeOptions, Default)]
pub(crate) struct MultiContainerOptions {
    dany: Option<ContainerOptions>,
    dbinary: Option<ContainerOptions>,
    ddebug: Option<ContainerOptions>,
    ddisplay: Option<ContainerOptions>,
    dlexp: Option<ContainerOptions>,
    dlhex: Option<ContainerOptions>,
    doctal: Option<ContainerOptions>,
    dpointer: Option<ContainerOptions>,
    duexp: Option<ContainerOptions>,
    duhex: Option<ContainerOptions>,
}

impl ContainerOptions {
    pub fn resolve<I>(attrs: I, attr_name: &str) -> syn::Result<Self>
    where
        I: IntoIterator<Item = Attribute>,
    {
        let attrs = DualAttr::collect(attrs, attr_name);
        let mut out = Self::default();

        for dattr in attrs {
            let opts = ContainerOptions::from_attr(dattr.attr)?;
            out += opts;
        }

        Ok(out)
    }
}

impl AddAssign for ContainerOptions {
    fn add_assign(&mut self, rhs: Self) {
        let Self {
            bounds: bounds_l,
            delegate_to: delegate_to_l,
        } = self;

        let Self {
            bounds: bounds_r,
            delegate_to: delegate_to_r,
        } = rhs;

        bounds_l.extend(bounds_r);

        if let Some(delegate_to) = delegate_to_r {
            *delegate_to_l = Some(delegate_to);
        }
    }
}

impl Add<ContainerOptions> for &ContainerOptions {
    type Output = ContainerOptions;

    fn add(self, rhs: ContainerOptions) -> Self::Output {
        let mut out = self.clone();
        out += rhs;
        out
    }
}

impl MultiContainerOptions {
    pub fn into_iter(self) -> impl FusedIterator<Item = (Alias<'static>, ContainerOptions)> {
        let Self {
            dany,
            dbinary,
            ddebug,
            ddisplay,
            dlexp,
            dlhex,
            doctal,
            dpointer,
            duexp,
            duhex,
        } = self;

        macro_rules! iter {
            ($default: ident | [$($id: ident),+ $(,)?] $(,)?) => {{
                let default = $default.unwrap_or_default();
                let arr = [$($id.map(|v| (Alias::$id, &default + v))),+];
                ::std::iter::IntoIterator::into_iter(arr)
            }};
        }

        let options = iter!(
            dany | [dbinary, ddebug, ddisplay, dlexp, dlhex, doctal, dpointer, duexp, duhex,]
        );

        options.flatten()
    }
}
