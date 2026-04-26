use crate::ATTR_ANY;
use impartial_ord::ImpartialOrd;
use std::cmp::Ordering;
use syn::Attribute;

#[derive(ImpartialOrd)]
pub(crate) struct DualAttr {
    pub attr_ty: AttrKind,
    pub attr: Attribute,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, ImpartialOrd)]
pub(crate) enum AttrKind {
    CatchAll,
    Primary,
}

impl AttrKind {
    pub fn aggregate<I>(attrs: I, attr_name: &str) -> Option<Self>
    where
        I: IntoIterator<Item = Attribute>,
    {
        DualAttr::collect(attrs, attr_name)
            .last()
            .map(move |a| a.attr_ty)
    }
}

impl DualAttr {
    pub fn collect<I>(attrs: I, attr_name: &str) -> Vec<Self>
    where
        I: IntoIterator<Item = Attribute>,
    {
        let mut out = attrs
            .into_iter()
            .filter_map(|a| Self::from_syn(a, attr_name))
            .collect::<Vec<_>>();

        out.sort();

        out
    }

    pub fn from_syn(attr: Attribute, attr_name: &str) -> Option<Self> {
        let ident = attr.path().get_ident()?;
        let ident_str = ident.to_string();

        Some(Self {
            attr_ty: if ident_str == attr_name {
                AttrKind::Primary
            } else if ident_str == ATTR_ANY {
                AttrKind::CatchAll
            } else {
                return None;
            },
            attr,
        })
    }
}

impl PartialEq for DualAttr {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.attr_ty == other.attr_ty
    }
}

impl Eq for DualAttr {}

#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for DualAttr {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.attr_ty.cmp(&other.attr_ty)
    }
}
