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
                    column: "author".to_owned(),
                    value: Shown::Person(PersonRef::Someone),
                },
                ShapedCell {
                    field: "text",
                    column: "text".to_owned(),
                    value: Shown::Text {
                        text: HOSTILE.to_owned(),
                        written: true,
                    },
                },
                ShapedCell {
                    field: "created_at",
                    column: "created_at".to_owned(),
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
fn priv_03_each_kind_is_named_in_plain_words_on_its_page_the_first_page_and_the_readme() {
    let file = written();
    // The plain name of the kind, not the doc comment of its entity.
    assert!(entry(&file, "comment.html").contains("<title>Comments</title>"));
    assert!(entry(&file, "comment.html").contains("<h1>Comments</h1>"));
    assert!(entry(&file, "index.html").contains("<a href=\"comment.html\">Comments</a>"));
    assert!(
        entry(&file, "README.txt").contains("== Comments (data/comment.json, comment.html) ==")
    );
    for (name, text) in &file {
        assert!(!text.contains("(ISS-05)"), "{name} names a feature");
    }
}

#[test]
fn m9_f91_each_column_is_headed_by_its_plain_title_not_by_the_code_or_its_doc_comment() {
    let file = written();
    let page = entry(&file, "comment.html");
    for header in ["Written by", "Text of the comment", "Written on"] {
        assert!(page.contains(&format!("<th>{header}</th>")), "{page}");
    }
    for code in ["<th>author</th>", "<th>text</th>", "<th>created_at</th>"] {
        assert!(!page.contains(code), "{page}");
    }
    assert!(!page.contains("in Markdown"), "a doc comment in a header");
    let readme = entry(&file, "README.txt");
    assert!(readme.contains("    author: Written by"), "{readme}");
    assert!(readme.contains("    created_at: Written on"), "{readme}");
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
    assert!(readme.contains("    text: Text of the comment"));
    assert!(
        kept_of(RecordKind::DataExport.code()).starts_with("7 "),
        "the files of exports are kept 7 days"
    );
}

#[test]
fn priv_03_the_readme_names_every_field_any_row_of_a_kind_holds() {
    let mut kind = comments();
    let mut only_text = kind.rows[0].clone();
    // The first row holds no text, the second nothing but its text.
    kind.rows[0].cells.retain(|cell| cell.field != "text");
    only_text.cells.retain(|cell| cell.field == "text");
    kind.rows.push(only_text);
    assert_eq!(fields_of(&kind), vec!["author", "created_at", "text"]);
    let mut writer = ExportWriter::new(Cursor::new(Vec::new()), Region::of(&Layers::default()));
    writer.kind(&kind).unwrap();
    let sink = writer.finish(&recipients(false, None)).unwrap();
    let mut archive = ZipArchive::new(Cursor::new(sink.into_inner())).unwrap();
    let mut readme = String::new();
    archive
        .by_name("README.txt")
        .unwrap()
        .read_to_string(&mut readme)
        .unwrap();
    assert!(readme.contains("    text: Text of the comment"), "{readme}");
}

#[test]
fn m9_f21_the_readme_reads_the_period_of_exports_from_the_keep_rules() {
    let period = rule(KeepKind::DataExports)
        .kept_for(&Layers::default())
        .render();
    let kept = kept_of(RecordKind::DataExport.code());
    assert!(kept.starts_with(&period), "{kept} names {period}");
    // A kind that no row deletes by its age is kept while it is needed.
    assert_ne!(kept_of("comment"), kept);
}

#[test]
fn priv_03_a_field_stored_in_several_columns_keeps_every_column_in_the_json() {
    let cell = |column: &str, value: Shown| ShapedCell {
        field: "spec",
        column: column.to_owned(),
        value,
    };
    let row = ShapedRow {
        links: vec![Link::Author],
        cells: vec![
            cell(
                "title",
                Shown::Text {
                    text: "Open bugs".to_owned(),
                    written: true,
                },
            ),
            cell("width", Shown::Number(6)),
            cell("time_span", Shown::Empty),
        ],
    };
    assert_eq!(
        row_json(&row)["spec"],
        serde_json::json!({"title": "Open bugs", "width": 6, "time_span": null})
    );
}
