//! Extension traits

use proc_macro2::{Ident, Punct, Spacing, Span};
use sealed::sealed;
use std::fmt::Display;
use syn::Error;

/// [`Ident`] extensions
#[sealed]
pub trait MacroificCoreIdentExt {
    /// Shorthand for `Ident::new(name, Span::call_site())`.
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_core::core_ext::*;
    /// # use proc_macro2::{Ident, Span};
    /// #
    /// let a = Ident::create("foo");
    /// let b = Ident::new("foo", Span::call_site());
    ///
    /// assert_eq!(a, b);
    /// ```
    fn create(name: &str) -> Self;
}

/// [`Punct`] extensions
#[sealed]
pub trait MacroificCorePunctExt {
    /// Create a new [`Punct`] with [`Spacing::Alone`].
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_core::core_ext::*;
    /// # use proc_macro2::{Punct, Spacing};
    /// #
    /// let p = Punct::new_alone('&');
    /// assert_eq!(p.spacing(), Spacing::Alone);
    /// assert_eq!(p.as_char(), '&');
    /// ```
    fn new_alone(ch: char) -> Self;

    /// Create a new [`Punct`] with [`Spacing::Joint`].
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_core::core_ext::*;
    /// # use proc_macro2::{Punct, Spacing};
    /// #
    /// let p = Punct::new_joint('^');
    /// assert_eq!(p.spacing(), Spacing::Joint);
    /// assert_eq!(p.as_char(), '^');
    /// ```
    fn new_joint(ch: char) -> Self;
}

/// [`Error`] extensions
#[sealed]
pub trait MacroificCoreErrorExt {
    /// Shorthand for [`Error::new(Span::call_site(), msg)`](Error::new).
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_core::core_ext::*;
    /// # use proc_macro2::Span;
    /// use syn::Error;
    ///
    /// let e1 = Error::call_site("msg").to_string();
    /// let e2 = Error::new(Span::call_site(), "msg").to_string();
    /// assert_eq!(e1, e2);
    /// ```
    fn call_site<T: Display>(msg: T) -> Self;
}

#[sealed]
impl MacroificCorePunctExt for Punct {
    #[inline]
    fn new_alone(ch: char) -> Self {
        Self::new(ch, Spacing::Alone)
    }

    #[inline]
    fn new_joint(ch: char) -> Self {
        Self::new(ch, Spacing::Joint)
    }
}

#[sealed]
impl MacroificCoreIdentExt for Ident {
    #[inline]
    fn create(name: &str) -> Self {
        Self::new(name, Span::call_site())
    }
}

#[sealed]
impl MacroificCoreErrorExt for Error {
    fn call_site<T: Display>(msg: T) -> Self {
        Self::new(Span::call_site(), msg)
    }
}
