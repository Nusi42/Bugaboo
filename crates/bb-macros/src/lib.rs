//! Derive macros for entities, personal-data marks and settings (ADR-0015, ADR-0018).
//!
//! `#[derive(Entity)]` is the build-time half of DATA-MARKS: a stored field without a
//! personal-data mark does not compile. The macro only reads the marks and writes them into a
//! constant; what the categories and legal bases are is defined once, in `bb_core::personal`.

#![forbid(unsafe_code)]

mod entity;
mod links;
mod masked;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Describes a stored entity to the personal-data map (ADR-0015).
///
/// Every field has to carry exactly one mark:
///
/// - `#[personal(category = "employee_time", basis = "legal_obligation")]` for personal data,
/// - `#[not_personal]` for everything else.
///
/// A field of a table that holds rows about many kinds of record (the domain events, the history)
/// may also say what it holds in the rows of one kind:
/// `#[personal_for(kind = "time_entry", category = "employee_time", basis = "contract")]`, once for
/// each kind. The kind is a word of `bb_core::event::RecordKind`.
///
/// A field without a mark, a mark with a missing or unknown part, or two marks on one field are
/// compile errors.
///
/// A field that names a person also says what the person is to the row (ADR-0044):
/// `#[person(author)]` (a word of `bb_core::personal::Link`) on a field that holds the ID of a
/// person, `#[person_for(record, link = "subject")]` or
/// `#[person_for(field_type = "person", key = "field", link = "subject")]` on a reference to a
/// record, `#[person_email(recipient)]` on an e-mail address, `#[person_in_changes]` on
/// the changes of a record, `#[person_in_json(subject)]` on JSON, and `#[names_no_person]` on a
/// reference or JSON that never names a person
/// (the module `links` says more). The links are the list the walk of a person's data reads;
/// DATA-MARKS fails a field that can name a person and has none. The macro implements `bb_core::personal::Entity`.
#[proc_macro_derive(
    Entity,
    attributes(
        personal,
        not_personal,
        personal_for,
        person,
        person_for,
        person_email,
        person_in_changes,
        person_in_json,
        names_no_person
    )
)]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    entity::expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Masks a view by the fields of the catalogue it holds, and the views inside it by their own
/// (SEC-04, M2-F21).
///
/// Every member of the struct says how it is masked, so none is forgotten:
///
/// - `#[masked(field = "title")]` for a member that holds a field of the catalogue (hidden when
///   the viewer's level for the field is *Hidden*),
/// - `#[masked(nested)]` for a view inside the view (or an `Option` or a `Vec` of them), which is
///   masked by its own `Masked` implementation,
/// - `#[masked(open)]` for a member that everybody who sees the view may see.
///
/// `field` and `nested` can be given together (a list of people in an *Assigned to* field). The
/// names of the members are the names of the fields, so a view with `#[serde(...)]` attributes is
/// refused. The macro implements `bb_policy::Masked`.
#[proc_macro_derive(Masked, attributes(masked))]
pub fn derive_masked(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    masked::expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
