//! Derive macros for entities, personal-data marks and settings (ADR-0015, ADR-0018).
//!
//! `#[derive(Entity)]` is the build-time half of DATA-MARKS: a stored field without a
//! personal-data mark does not compile. The macro only reads the marks and writes them into a
//! constant; what the categories and legal bases are is defined once, in `bb_core::personal`.

#![forbid(unsafe_code)]

mod entity;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Describes a stored entity to the personal-data map (ADR-0015).
///
/// Every field has to carry exactly one mark:
///
/// - `#[personal(category = "employee_time", basis = "legal_obligation")]` for personal data,
/// - `#[not_personal]` for everything else.
///
/// A field without a mark, a mark with a missing or unknown part, or two marks on one field are
/// compile errors. The macro implements `bb_core::personal::Entity`.
#[proc_macro_derive(Entity, attributes(personal, not_personal))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    entity::expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
