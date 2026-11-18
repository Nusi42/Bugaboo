//! `README.txt` of the file (ADR-0044, decision 2): for each kind of record the information of
//! GDPR Art. 15(1), generated and never written by hand: the purpose of each category of its
//! personal fields (`bb_core::personal::Category`), their legal bases, how long the kind is kept
//! and who receives it (the outside services that are switched on), and what each field means.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use bb_core::message;
use bb_core::personal::{EntityMarks, FieldMark, Personal};

use super::pages::{label_of, title_of, words};

/// The categories and legal bases of the personal fields of `marks`, each once, in the order of
/// their words.
fn marks_of(marks: &EntityMarks) -> (Vec<String>, Vec<String>) {
    let mut purposes = BTreeSet::new();
    let mut bases = BTreeSet::new();
    let mut note = |personal: Personal| {
        if let Personal::Yes { category, basis } = personal {
            purposes.insert(category);
            bases.insert(basis);
        }
    };
    for field in marks.fields() {
        note(field.personal_data());
        for mark in field.by_kind() {
            note(mark.personal_data());
        }
    }
    (
        purposes
            .into_iter()
            .map(|category| words(category.purpose()))
            .collect(),
        bases.into_iter().map(|basis| words(basis.name())).collect(),
    )
}

/// One line of a field: its name in the code (the key in the JSON) and what it means.
fn field_line(field: &FieldMark) -> String {
    format!("    {}: {}", field.name(), label_of(field))
}

/// What the README says of one kind of record. `fields` are the fields that went into the file.
pub(super) struct ReadmeKind<'a> {
    /// The kind: the name of the table, of its page and of its JSON.
    pub kind: &'a str,
    /// The marks and links of the entity.
    pub marks: &'a EntityMarks,
    /// The fields in the file.
    pub fields: &'a [&'static str],
    /// How long it is kept, in words.
    pub kept: String,
}

/// The text of `README.txt`: the kinds of record of the file and who receives the data.
#[must_use]
pub(super) fn readme(kinds: &[ReadmeKind<'_>], recipients: &[String]) -> String {
    let mut text = format!(
        "{}\n\n{}\n\n{}\n",
        words(message::EXPORT_README_TITLE),
        words(message::EXPORT_README_INTRO),
        words(message::EXPORT_README_FILES),
    );
    for kind in kinds {
        let (purposes, bases) = marks_of(kind.marks);
        let _ = write!(
            text,
            "\n== {} (data/{}.json, {}.html) ==\n",
            title_of(kind.marks),
            kind.kind,
            kind.kind
        );
        if purposes.is_empty() {
            let _ = writeln!(text, "{}", words(message::EXPORT_README_NOT_PERSONAL));
        } else {
            let _ = writeln!(
                text,
                "{}: {}",
                words(message::EXPORT_README_PURPOSE),
                purposes.join(" ")
            );
            let _ = writeln!(
                text,
                "{}: {}",
                words(message::EXPORT_README_BASIS),
                bases.join("; ")
            );
        }
        let _ = writeln!(
            text,
            "{}: {}",
            words(message::EXPORT_README_KEPT),
            kind.kept
        );
        let _ = writeln!(
            text,
            "{}: {}",
            words(message::EXPORT_README_RECIPIENTS),
            recipients.join(" ")
        );
        let _ = writeln!(text, "{}:", words(message::EXPORT_README_FIELDS));
        for field in kind
            .marks
            .fields()
            .iter()
            .filter(|field| kind.fields.contains(&field.name()))
        {
            let _ = writeln!(text, "{}", field_line(field));
        }
    }
    text
}

/// Who receives the data, in words: the people of the organization as their rights allow, the mail
/// server when e-mail is switched on, and the AI provider with its country when the assistant is.
#[must_use]
pub fn recipients(mail_on: bool, ai: Option<(&str, &str)>) -> Vec<String> {
    let mut lines = vec![words(message::EXPORT_README_RECIPIENT_PEOPLE)];
    if mail_on {
        lines.push(words(message::EXPORT_README_RECIPIENT_MAIL));
    }
    if let Some((provider, country)) = ai {
        lines.push(message::export_readme_recipient_ai(country, provider).render());
    }
    lines
}
