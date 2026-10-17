//! File adapter: content-addressed storage, type checks, previews and scanning (ADR-0032).
//!
//! [`DiskFiles`] keeps the bytes of files in the `files/` folder of the data folder: uploads are
//! put together part by part, a finished file is stored once under the SHA-256 of its content,
//! and the real type of a file is read from its content ([`sniff`]), never from its name. A preview
//! is made by the program itself in a child process with limits ([`preview`]), so a crafted file
//! can crash the child and nothing else. The adapter implements the port `FileStore` of `bb-app`;
//! a second adapter (S3-compatible storage, ATT-06) can come later.

#![forbid(unsafe_code)]

mod disk;
#[cfg(any(test, feature = "testing"))]
pub mod fixtures;
pub mod preview;
mod sniff;

pub use disk::DiskFiles;
pub use preview::{PreviewCommand, PreviewRefusal};
pub use sniff::sniff;

/// The folder of the data folder in which the files live. The server never serves it directly: a
/// file is only sent through a signed link (ADR-0032, ATT-02).
pub const FILES_FOLDER: &str = "files";
