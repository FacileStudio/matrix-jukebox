//! Utilities for extracting specific types of fields.

use proc_macro2::Span;
use sealed::sealed;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Data, DataEnum, DataStruct, DataUnion, Error, Field, Fields, FieldsNamed, FieldsUnnamed, Token,
};

type PunctuatedFields = Punctuated<Field, Token![,]>;

/// Convert this rejection to/into a [`syn::Error`].
#[sealed]
pub trait ToSynError {
    /// Convert this rejection to a [`syn::Error`].
    fn to_syn_err(&self) -> Error;

    /// Convert this rejection into a [`syn::Error`].
    #[inline]
    fn into_syn_err(self) -> Error
    where
        Self: Sized,
    {
        self.to_syn_err()
    }
}

/// [`Data`] extensions.
#[sealed]
pub trait DataExtractExt {
    /// Extract a union from a [`DeriveInput`](syn::DeriveInput)'s data.
    ///
    /// # Errors
    /// If there's a container mismatch.
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_core::extract_fields::*;
    /// # use syn::{DeriveInput, parse_quote};
    ///
    /// let input: DeriveInput = parse_quote!(struct MyStruct;);
    /// let err: syn::Error = input.data.extract_union().unwrap_err().into();
    /// assert_eq!(err.to_string(), "Only unions supported");
    ///
    /// let input: DeriveInput = parse_quote!(union MyUnion { foo: u8 });
    /// input.data.extract_union().unwrap(); // Ok
    /// ```
    fn extract_union(self) -> Result<DataUnion, Rejection<DataStruct, DataEnum>>;

    /// Extract a struct from a [`DeriveInput`](syn::DeriveInput)'s data.
    ///
    /// # Errors
    /// If there's a container mismatch.
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_core::extract_fields::*;
    /// # use syn::{DeriveInput, parse_quote};
    ///
    /// let input: DeriveInput = parse_quote!(enum MyEnum { Foo });
    /// let err: syn::Error = input.data.extract_struct().unwrap_err().into();
    /// assert_eq!(err.to_string(), "Only structs supported");
    ///
    /// let input: DeriveInput = parse_quote!(struct MyStruct;);
    /// input.data.extract_struct().unwrap(); // Ok
    /// ```
    fn extract_struct(self) -> Result<DataStruct, Rejection<DataEnum, DataUnion>>;

    /// Extract an enum from a [`DeriveInput`](syn::DeriveInput)'s data.
    ///
    /// # Errors
    /// If there's a container mismatch.
    ///
    /// ```
    /// # use macroific_core::extract_fields::*;
    /// # use syn::{DeriveInput, parse_quote};
    ///
    /// let input: DeriveInput = parse_quote!(struct MyStruct;);
    /// let err: syn::Error = input.data.extract_enum().unwrap_err().into();
    /// assert_eq!(err.to_string(), "Only enums supported");
    ///
    /// let input: DeriveInput = parse_quote!(enum MyEnum { Foo });
    /// input.data.extract_enum().unwrap(); // Ok
    /// ```
    fn extract_enum(self) -> Result<DataEnum, Rejection<DataStruct, DataUnion>>;

    /// [`extract_struct`](crate::extract_fields::DataExtractExt::extract_struct) and then
    /// [`extract_any_fields`](FieldsExtractExt::extract_any_fields).
    ///
    /// # Errors
    /// If there's a container mismatch.
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_core::extract_fields::*;
    /// # use syn::{DeriveInput, Fields, parse_quote};
    /// #
    /// let input: DeriveInput = parse_quote!(struct Foo { foo: u8 });
    /// assert!(input.data.extract_struct_fields().is_ok());
    ///
    /// let input: DeriveInput = parse_quote!(struct Foo;);
    /// let err: syn::Error = input.data.extract_struct_fields().unwrap_err().into();
    /// assert_eq!(err.to_string(), "Unit structs not supported");
    ///
    /// let input: DeriveInput = parse_quote!(enum Foo { A(u8) });
    /// let err: syn::Error = input.data.extract_struct_fields().unwrap_err().into();
    /// assert_eq!(err.to_string(), "Only structs supported");
    /// ```
    fn extract_struct_fields(self) -> syn::Result<PunctuatedFields>
    where
        Self: Sized,
    {
        self.extract_struct()?.fields.extract_any_fields()
    }

    /// [`extract_struct`](DataExtractExt::extract_struct) and then
    /// [`extract_named_fields`](FieldsExtractExt::extract_named_fields).
    ///
    /// # Errors
    /// If there's a container mismatch.
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_core::extract_fields::*;
    /// # use syn::{DeriveInput, Fields, parse_quote};
    /// #
    /// let input: DeriveInput = parse_quote!(struct Foo { foo: u8 });
    /// assert!(input.data.extract_struct_named().is_ok());
    ///
    /// let input: DeriveInput = parse_quote!(struct Foo(u8););
    /// let err: syn::Error = input.data.extract_struct_named().unwrap_err().into();
    /// assert_eq!(err.to_string(), "Only named fields supported");
    ///
    /// let input: DeriveInput = parse_quote!(enum Foo { A(u8) });
    /// let err: syn::Error = input.data.extract_struct_named().unwrap_err().into();
    /// assert_eq!(err.to_string(), "Only structs supported");
    /// ```
    fn extract_struct_named(self) -> syn::Result<PunctuatedFields>
    where
        Self: Sized,
    {
        self.extract_struct()
            .map_err(Error::from)?
            .fields
            .extract_named_fields()
            .map_err(Error::from)
    }

    /// [`extract_struct`](DataExtractExt::extract_struct) and then
    /// [`extract_unnamed_fields`](FieldsExtractExt::extract_unnamed_fields).
    ///
    /// # Errors
    /// If there's a container mismatch.
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_core::extract_fields::*;
    /// # use syn::{DeriveInput, Fields, parse_quote};
    /// #
    /// let input: DeriveInput = parse_quote!(struct Foo(u8););
    /// assert!(input.data.extract_struct_unnamed().is_ok());
    ///
    /// let input: DeriveInput = parse_quote!(struct Foo { foo: u8 });
    /// let err: syn::Error = input.data.extract_struct_unnamed().unwrap_err().into();
    /// assert_eq!(err.to_string(), "Only unnamed fields supported");
    ///
    /// let input: DeriveInput = parse_quote!(enum Foo { A(u8) });
    /// let err: syn::Error = input.data.extract_struct_unnamed().unwrap_err().into();
    /// assert_eq!(err.to_string(), "Only structs supported");
    /// ```
    fn extract_struct_unnamed(self) -> syn::Result<PunctuatedFields>
    where
        Self: Sized,
    {
        self.extract_struct()
            .map_err(Error::from)?
            .fields
            .extract_unnamed_fields()
            .map_err(Error::from)
    }
}

#[sealed]
impl DataExtractExt for Data {
    fn extract_union(self) -> Result<DataUnion, Rejection<DataStruct, DataEnum>> {
        match self {
            Data::Struct(data) => Err(Rejection::A(data)),
            Data::Enum(data) => Err(Rejection::B(data)),
            Data::Union(data) => Ok(data),
        }
    }

    fn extract_struct(self) -> Result<DataStruct, Rejection<DataEnum, DataUnion>> {
        match self {
            Data::Struct(data) => Ok(data),
            Data::Enum(data) => Err(Rejection::A(data)),
            Data::Union(data) => Err(Rejection::B(data)),
        }
    }

    fn extract_enum(self) -> Result<DataEnum, Rejection<DataStruct, DataUnion>> {
        match self {
            Data::Struct(data) => Err(Rejection::A(data)),
            Data::Enum(data) => Ok(data),
            Data::Union(data) => Err(Rejection::B(data)),
        }
    }
}

/// [`Fields`] extensions
#[sealed]
pub trait FieldsExtractExt {
    /// Extract named fields from [`Fields`]. `()` is returned for unit structs.
    ///
    /// # Errors
    /// If the fields are unnamed, `Err(Rejection::A)` is returned. If the fields are unit,
    /// `Err(Rejection::B)` is returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_core::extract_fields::*;
    /// # use syn::{Fields, parse_quote};
    /// #
    /// let input = Fields::Unnamed(parse_quote!((u8)));
    /// let err: syn::Error = input.extract_named_fields().unwrap_err().into();
    ///
    /// assert_eq!(err.to_string(), "Only named fields supported");
    ///
    /// let input = Fields::Named(parse_quote!({ bar: u8 }));
    /// input.extract_named_fields().unwrap(); // Ok
    /// ```
    fn extract_named_fields(self) -> Result<PunctuatedFields, Rejection<FieldsUnnamed, ()>>;

    /// Extract unnamed fields from [`Fields`]. `()` is returned for unit structs.
    ///
    /// # Errors
    /// If the fields are named, `Err(Rejection::A)` is returned. If the fields are unit,
    /// `Err(Rejection::B)` is returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_core::extract_fields::*;
    /// # use syn::{Fields, parse_quote};
    /// #
    /// let input = Fields::Named(parse_quote!({ bar: u8 }));
    /// let err: syn::Error = input.extract_unnamed_fields().unwrap_err().into();
    /// assert_eq!(err.to_string(), "Only unnamed fields supported");
    ///
    /// let input = Fields::Unnamed(parse_quote!((u8)));
    /// input.extract_unnamed_fields().unwrap(); // Ok
    /// ```
    fn extract_unnamed_fields(self) -> Result<PunctuatedFields, Rejection<FieldsNamed, ()>>;

    /// Extract named or unnamed fields
    ///
    /// # Errors
    /// If it's a unit struct
    ///
    /// # Example
    ///
    /// ```
    /// # use macroific_core::extract_fields::*;
    /// # use syn::Fields;
    /// #
    /// let err: syn::Error = Fields::Unit.extract_any_fields().unwrap_err().into();
    /// assert_eq!(err.to_string(), "Unit structs not supported");
    /// ```
    fn extract_any_fields(self) -> syn::Result<PunctuatedFields>
    where
        Self: Sized,
    {
        match self.extract_named_fields() {
            Ok(fields) => Ok(fields),
            Err(Rejection::A(fields)) => Ok(fields.unnamed),
            Err(e) => Err(Error::new(
                e.into_syn_err().span(),
                "Unit structs not supported",
            )),
        }
    }
}

#[sealed]
impl FieldsExtractExt for Fields {
    /// Extract named fields from [`Fields`]. `()` is returned for unit structs.
    fn extract_named_fields(self) -> Result<PunctuatedFields, Rejection<FieldsUnnamed, ()>> {
        match self {
            Fields::Named(fields) => Ok(fields.named),
            Fields::Unnamed(fields) => Err(Rejection::A(fields)),
            Fields::Unit => Err(Rejection::B(())),
        }
    }

    /// Extract unnamed fields from [`Fields`]. `()` is returned for unit structs.
    fn extract_unnamed_fields(self) -> Result<PunctuatedFields, Rejection<FieldsNamed, ()>> {
        match self {
            Fields::Named(fields) => Err(Rejection::A(fields)),
            Fields::Unnamed(fields) => Ok(fields.unnamed),
            Fields::Unit => Err(Rejection::B(())),
        }
    }
}

/// One of two error states
#[allow(missing_docs)]
#[derive(Debug, Eq, PartialEq)]
pub enum Rejection<A, B> {
    A(A),
    B(B),
}

impl<A, B> From<Rejection<A, B>> for Error
where
    Rejection<A, B>: ToSynError,
{
    #[inline]
    fn from(value: Rejection<A, B>) -> Self {
        value.into_syn_err()
    }
}

#[sealed]
impl ToSynError for Rejection<FieldsUnnamed, ()> {
    fn to_syn_err(&self) -> Error {
        Error::new(
            match *self {
                Self::A(ref f) => f.span(),
                Self::B(()) => Span::call_site(),
            },
            "Only named fields supported",
        )
    }
}

#[sealed]
impl ToSynError for Rejection<FieldsNamed, ()> {
    fn to_syn_err(&self) -> Error {
        Error::new(
            match *self {
                Self::A(ref f) => f.span(),
                Self::B(()) => Span::call_site(),
            },
            "Only unnamed fields supported",
        )
    }
}

macro_rules! impl_reject {
    ($msg: literal => [$a: ty => $p_a: ident, $b: ty => $p_b: ident]) => {
        #[sealed]
        impl ToSynError for Rejection<$a, $b> {
            fn to_syn_err(&self) -> Error {
                Error::new(
                    match self {
                        Self::A(v) => v.$p_a.span(),
                        Self::B(v) => v.$p_b.span(),
                    },
                    $msg,
                )
            }
        }
    };
}

impl_reject!("Only structs supported" => [DataEnum => enum_token, DataUnion => union_token]);
impl_reject!("Only enums supported" => [DataStruct => struct_token, DataUnion => union_token]);
impl_reject!("Only unions supported" => [DataStruct => struct_token, DataEnum => enum_token]);
