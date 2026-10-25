//! Searching (QRY-04, QRY-05, SEC-17): what a person may ask a search about. The compiler and the
//! routes of the search build on this module.

mod readable;

pub use readable::{ReadableFieldsQuery, SearchFields, SearchScope, readable_fields};
