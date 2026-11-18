//! The file of *Download my data* as it is written (ADR-0044, decision 2).

use std::io::{Cursor, Read as _};

use bb_core::comment::Comment;
use bb_core::personal::{Entity as _, Link};
use bb_core::settings::Layers;
use bb_core::wording::Region;
use bb_policy::PersonRef;
use zip::ZipArchive;

use super::*;
use crate::privacy::export::readme::recipients;
use crate::privacy::export::shape::{ShapedCell, ShapedKind, ShapedRow, Shown};

/// A text that would run a script or load something if a page took it as it is.
const HOSTILE: &str = "<script>alert(1)</script> [click](javascript:alert(1)) \
                       <img src=x onerror=alert(1)> ![p](https://tracker.example/p.png)";

fn comments() -> ShapedKind {
    ShapedKind {
        kind: "comment".to_owned(),
        marks: Comment::MARKS,
        rows: vec![ShapedRow {
            links: vec![Link::Author],
            cells: vec![
                ShapedCell {
                    field: "author",
                    value: Shown::Person(PersonRef::Someone),
                },
                ShapedCell {
                    field: "text",
                    value: Shown::Text {
                        text: HOSTILE.to_owned(),
                        written: true,
                    },
                },
                ShapedCell {
                    field: "created_at",
                    value: Shown::Moment(jiff::Timestamp::from_second(1_767_600_000).unwrap()),
                },
            ],
        }],
    }
}

/// The file with the comments, as name and text of each entry.
fn written() -> Vec<(String, String)> {
    let mut writer = ExportWriter::new(Cursor::new(Vec::new()), Region::of(&Layers::default()));
    writer.kind(&comments()).unwrap();
    writer.start_upload("0190aa").unwrap();
    writer.write_bytes(b"bytes of a file").unwrap();
    let sink = writer
        .finish(&recipients(true, Some(("Anthropic", "USA"))))
        .unwrap();
    let mut archive = ZipArchive::new(Cursor::new(sink.into_inner())).unwrap();
    (0..archive.len())
        .map(|at| {
            let mut entry = archive.by_index(at).unwrap();
            let mut text = String::new();
            entry.read_to_string(&mut text).unwrap();
            (entry.name().unwrap().into_owned(), text)
        })
        .collect()
}

fn entry<'a>(file: &'a [(String, String)], name: &str) -> &'a str {
    &file
        .iter()
        .find(|(found, _)| found == name)
        .unwrap_or_else(|| panic!("no {name} in the file"))
        .1
}

#[test]
fn priv_03_the_file_holds_readable_pages_and_the_same_rows_as_json() {
    let file = written();
    let names: Vec<&str> = file.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "data/comment.json",
            "comment.html",
            "files/0190aa",
            "index.html",
            "README.txt"
        ]
    );
    let rows: serde_json::Value = serde_json::from_str(entry(&file, "data/comment.json")).unwrap();
    assert_eq!(rows[0]["what_you_are"], serde_json::json!(["author"]));
    assert_eq!(rows[0]["text"], HOSTILE);
    assert_eq!(rows[0]["author"]["shown"], "someone");
    assert_eq!(
        rows[0]["created_at"], "2026-01-05T08:00:00Z",
        "a moment in UTC in the JSON"
    );
    let page = entry(&file, "comment.html");
    assert!(page.contains("var(--bg-canvas)") && page.contains("--bg-canvas:#ffffff"));
    assert!(
        !page.contains("2026-01-05T08:00:00Z"),
        "a date in the person's Region on the page"
    );
    assert!(entry(&file, "index.html").contains("href=\"comment.html\""));
    assert_eq!(entry(&file, "files/0190aa"), "bytes of a file");
}

#[test]
fn priv_03_the_pages_hold_no_script() {
    let file = written();
    for (name, text) in file.iter().filter(|(name, _)| {
        std::path::Path::new(name)
            .extension()
            .is_some_and(|ext| ext == "html")
    }) {
        let lower = text.to_lowercase();
        for banned in [
            "<script",
            "href=\"javascript:",
            "<img",
            "<iframe",
            "<object",
        ] {
            assert!(!lower.contains(banned), "{name} holds {banned}");
        }
        // No tag carries an event handler: the hostile text is only ever text.
        for tag in lower
            .split('<')
            .skip(1)
            .filter_map(|rest| rest.split('>').next())
        {
            assert!(!tag.contains(" on"), "{name} has the tag <{tag}>");
        }
        assert!(
            !lower.contains("tracker.example/p.png\""),
            "{name} loads a picture from outside"
        );
    }
}

#[test]
fn priv_03_the_readme_gives_purpose_basis_keep_period_and_recipients_for_every_kind() {
    let file = written();
    let readme = entry(&file, "README.txt");
    assert!(readme.contains("data/comment.json"));
    for (category, basis) in [
        (
            bb_core::personal::Category::Activity,
            bb_core::personal::Basis::Contract,
        ),
        (
            bb_core::personal::Category::Content,
            bb_core::personal::Basis::Contract,
        ),
    ] {
        assert!(readme.contains(&words(category.purpose())));
        assert!(readme.contains(&words(basis.name())));
    }
    assert!(readme.contains(&words(message::EXPORT_README_KEPT_WHILE_NEEDED)));
    for line in recipients(true, Some(("Anthropic", "USA"))) {
        assert!(readme.contains(&line), "the README names {line}");
    }
    assert!(readme.contains("Anthropic (USA)"));
    assert_eq!(
        recipients(false, None).len(),
        1,
        "only the people of the organization"
    );
    assert!(readme.contains("    text: The text, in Markdown."));
    assert!(
        kept_of(RecordKind::DataExport.code()).starts_with("7 "),
        "the files of exports are kept 7 days"
    );
}
