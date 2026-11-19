//! The ZIP file of *Download my data* (ADR-0044, decision 2): `index.html`, one page and one
//! `data/<kind>.json` for each kind of record, `files/` with the bytes of the files the person
//! uploaded, and `README.txt`. Each kind is written whole before the next, so no more than one
//! kind is held in memory; no name in the file comes from what a person typed.

use std::io::{self, Seek, Write};

use bb_core::data_export::EXPORT_KEPT;
use bb_core::event::RecordKind;
use bb_core::message;
use bb_core::personal::EntityMarks;
use bb_core::wording::Region;
use serde_json::{Map, Value as Json, json};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::pages::{IndexLine, index_page, kind_page, title_of, words};
use super::readme::{ReadmeKind, readme};
use super::shape::{ShapedKind, ShapedRow, Shown};

/// The key of a row's links in the JSON: what the person is to the row.
const LINKS_KEY: &str = "what_you_are";

/// What a kind of record that was written leaves for the README.
struct Written {
    kind: String,
    marks: EntityMarks,
    fields: Vec<&'static str>,
}

/// How long `kind` is kept, in words. Until the keep and delete rules give a period for every kind
/// (follow-up M9-F21), only the files of this export have one of their own.
fn kept_of(kind: &str) -> String {
    if kind == RecordKind::DataExport.code() {
        message::export_readme_kept_for_days(EXPORT_KEPT.as_hours() / 24).render()
    } else {
        words(message::EXPORT_README_KEPT_WHILE_NEEDED)
    }
}

/// A value in the JSON: moments in UTC, people as `PersonRef`.
fn json_of(value: &Shown) -> Json {
    match value {
        Shown::Empty => Json::Null,
        Shown::Number(number) => json!(number),
        Shown::Decimal(number) => json!(number),
        Shown::Truth(truth) => json!(truth),
        Shown::Moment(moment) => json!(moment.to_string()),
        Shown::Text { text, .. } | Shown::Id(text) | Shown::Bytes(text) => json!(text),
        Shown::Person(person) => serde_json::to_value(person).unwrap_or(Json::Null),
    }
}

/// A row in the JSON.
fn row_json(row: &ShapedRow) -> Json {
    let mut object = Map::new();
    object.insert(
        LINKS_KEY.to_owned(),
        json!(row.links.iter().map(|link| link.code()).collect::<Vec<_>>()),
    );
    for cell in &row.cells {
        object.insert(cell.field.to_owned(), json_of(&cell.value));
    }
    Json::Object(object)
}

/// The rows of a kind in the JSON.
#[must_use]
fn kind_json(kind: &ShapedKind) -> Json {
    Json::Array(kind.rows.iter().map(row_json).collect())
}

/// The fields the rows of a kind hold, each once, in the order they first come: a row may hold a
/// field another row leaves out, and the README names every one.
fn fields_of(kind: &ShapedKind) -> Vec<&'static str> {
    let mut fields = Vec::new();
    for cell in kind.rows.iter().flat_map(|row| &row.cells) {
        if !fields.contains(&cell.field) {
            fields.push(cell.field);
        }
    }
    fields
}

fn failed(error: zip::result::ZipError) -> io::Error {
    io::Error::other(error)
}

/// The file being written.
pub struct ExportWriter<W: Write + Seek> {
    zip: ZipWriter<W>,
    region: Region,
    lines: Vec<IndexLine>,
    written: Vec<Written>,
}

impl<W: Write + Seek> ExportWriter<W> {
    /// A file written into `sink`, with the dates of its pages in `region`.
    pub fn new(sink: W, region: Region) -> Self {
        Self {
            zip: ZipWriter::new(sink),
            region,
            lines: Vec::new(),
            written: Vec::new(),
        }
    }

    fn start(&mut self, name: &str) -> io::Result<()> {
        self.zip
            .start_file(
                name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .map_err(failed)
    }

    /// Writes one kind of record: its JSON and its page.
    ///
    /// # Errors
    /// The error of the sink.
    pub fn kind(&mut self, kind: &ShapedKind) -> io::Result<()> {
        self.start(&format!("data/{}.json", kind.kind))?;
        serde_json::to_writer_pretty(&mut self.zip, &kind_json(kind))?;
        self.start(&format!("{}.html", kind.kind))?;
        self.zip
            .write_all(kind_page(kind, &self.region).as_bytes())?;
        self.lines.push(IndexLine {
            kind: kind.kind.clone(),
            title: title_of(&kind.marks),
            rows: kind.rows.len(),
        });
        self.written.push(Written {
            kind: kind.kind.clone(),
            marks: kind.marks,
            fields: fields_of(kind),
        });
        Ok(())
    }

    /// Starts a file the person uploaded, named `files/<name>`; its bytes follow with
    /// [`Self::write_bytes`].
    ///
    /// # Errors
    /// The error of the sink.
    pub fn start_upload(&mut self, name: &str) -> io::Result<()> {
        self.start(&format!("files/{name}"))
    }

    /// Writes the next bytes of the file that was started last.
    ///
    /// # Errors
    /// The error of the sink.
    pub fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.zip.write_all(bytes)
    }

    /// Writes the first page and the README and closes the file; `recipients` say who receives
    /// the data, in words.
    ///
    /// # Errors
    /// The error of the sink.
    pub fn finish(mut self, recipients: &[String]) -> io::Result<W> {
        self.start("index.html")?;
        self.zip.write_all(index_page(&self.lines).as_bytes())?;
        let kinds: Vec<ReadmeKind<'_>> = self
            .written
            .iter()
            .map(|written| ReadmeKind {
                kind: &written.kind,
                marks: &written.marks,
                fields: &written.fields,
                kept: kept_of(&written.kind),
            })
            .collect();
        let text = readme(&kinds, recipients);
        self.start("README.txt")?;
        self.zip.write_all(text.as_bytes())?;
        self.zip.finish().map_err(failed)
    }
}

#[cfg(test)]
#[path = "tests/writer.rs"]
mod tests;
