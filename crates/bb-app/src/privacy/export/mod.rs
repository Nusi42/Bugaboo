//! *Download my data* (PRIV-03, ADR-0044, decisions 2 to 4): the records of the files, the job that
//! writes one, and what goes into it.

mod pages;
mod readme;
mod shape;
mod store;
mod writer;

pub use readme::recipients;
pub use shape::{ShapedCell, ShapedKind, ShapedRow, Shown, people_in, shape_row};
pub use store::ExportStore;
pub use writer::ExportWriter;
