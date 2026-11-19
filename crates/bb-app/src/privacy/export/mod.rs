//! *Download my data* (PRIV-03, ADR-0044, decisions 2 to 4): the records of the files, the job that
//! writes one, and what goes into it.

mod ask;
mod files;
mod issue_fields;
mod job;
mod pages;
mod readme;
mod shape;
mod store;
mod types;
mod writer;

pub use ask::{
    EXPORTS_PAGE_MOST, ask_for_my_data, ask_for_someones_data, data_exports, download_export,
    export_link, my_data_export,
};
pub use files::{ExportFiles, ExportSink, NoExportFiles};
pub(crate) use pages::title_of;
pub use readme::recipients;
pub use shape::{ShapedCell, ShapedKind, ShapedRow, Shown, people_in, shape_row};
pub use store::ExportStore;
pub use types::*;
pub use writer::ExportWriter;
