//! Own fields (CF-01, CF-03, CF-04, CF-06, CF-07, CF-08): the fields an organization adds to
//! issues and to projects.
//!
//! The use cases are plain `async fn`s that take the context and an input (`types.rs`): the
//! catalogue of fields (`catalogue.rs`), making and changing a field (`manage.rs`), the choices of
//! a field (`choices.rs`) and the values items hold (`values.rs`, used by the use cases that make
//! and change issues and projects). They depend on storage only through the port
//! [`FieldStore`] (`store.rs`) and on the policy engine for every decision.
//!
//! Three rules run through all of them:
//! - **A hidden field does not exist for the person**: its definition, its values and its changes
//!   are in no answer, and asking for it answers like a field that is not there (CF-06).
//! - **A change is made on the version that was looked at**: the version of the field for its
//!   definition and its choices, the version of the item for its values.
//! - **The same rules, once**: `bb_core::own_fields_value::Rules::check` decides a value, here
//!   and (without the pattern) in the browser.

mod catalogue;
mod choices;
mod manage;
pub(crate) mod member;
mod pattern;
mod store;
mod text;
mod types;
mod values;

pub use catalogue::{field, field_types, field_usage, fields};
pub use choices::{add_choice, change_choice, combine_choices, reorder_choices};
pub(crate) use manage::must_be_asked_for;
pub use manage::{change_field, create_field};
pub use store::{FieldStore, StoredValues};
pub use text::{SaveFieldText, save_field_text};
pub use types::*;

pub(crate) use values::{Item, ItemValues, PlannedValues, is_kept};
