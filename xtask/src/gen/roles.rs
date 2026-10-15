//! The tables of permissions and starting roles in architecture §7, written from the code of
//! `bb-policy` (ADR-0025: the code is the source, never the page).
//!
//! Five tables of the page are generated: the field catalogue, the permission catalogue and, for the
//! six starting roles, their permissions, field levels and *Who sees people* levels. Each sits between a
//! `BEGIN GENERATED` and an `END GENERATED` comment of the page; the text around them is written by
//! hand. DOC-GENERATED fails when a table differs from what this makes, so nobody edits one by hand.

use anyhow::{Context as _, bail};
use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::fields::{BelongsTo, Field};
use bb_core::message::{Message, MessageKey};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Permission, StartingRole};

use super::Generated;
use crate::repo::Repo;
use crate::source::read_text;

/// The page that holds the tables.
pub(crate) const PAGE: &str = "docs/architecture/07-cross-cutting.md";

/// What stands before the name of a table in its first comment.
const BEGIN: &str = "<!-- BEGIN GENERATED ";
/// What stands before the name of a table in its last comment.
const END: &str = "<!-- END GENERATED ";
/// What the first comment says besides the name.
const NOTE: &str =
    "by `cargo xtask gen roles` from bb-core and bb-policy; edit the code, not this table -->";

/// The text of a message.
fn text(key: MessageKey) -> String {
    Message::from(key).render()
}

/// A line of a Markdown table.
fn row(cells: &[String]) -> String {
    format!("| {} |\n", cells.join(" | "))
}

/// The head of a table whose first column is `first` and whose other columns are the starting
/// roles.
fn role_head(first: &str, align: &str) -> String {
    let mut cells = vec![first.to_owned()];
    cells.extend(StartingRole::ALL.iter().map(|role| text(role.name())));
    format!(
        "{}|---|{}|\n",
        row(&cells),
        vec![align; StartingRole::ALL.len()].join("|")
    )
}

/// The word with a capital.
fn capital(word: &str) -> String {
    let mut letters = word.chars();
    letters
        .next()
        .map(|first| first.to_uppercase().chain(letters).collect())
        .unwrap_or_default()
}

/// What a field belongs to, in the words of the catalogue table.
fn belongs_to(thing: BelongsTo) -> &'static str {
    match thing {
        BelongsTo::Issue => "issue",
        BelongsTo::IssueTimeTotal => "issue (total of its time entries)",
        BelongsTo::TimeEntry => "time entry (default on the issue)",
        BelongsTo::TimeEntryWithProjectDefault => "time entry (default on the issue and project)",
    }
}

/// The fields of the catalogue: the key, the name, what it belongs to and the kind of people
/// information when it holds a person.
fn field_catalogue() -> String {
    let mut out = String::from(
        "| Key | Name | Belongs to | Holds a person |
|-----|------|-----------|----------------|
",
    );
    for field in Field::ALL {
        out.push_str(&row(&[
            format!("`{}`", field.code()),
            text(field.name()),
            belongs_to(field.belongs_to()).to_owned(),
            field.person_kind().map_or_else(
                || "–".to_owned(),
                |kind| format!("yes, kind *{}*", text(kind.name())),
            ),
        ]));
    }
    out
}

/// The permission catalogue.
fn permission_catalogue() -> String {
    let mut out = String::from(
        "| Key | Topic | Counts | Kind | Sentence on the role page |\n|-----|-------|--------|------|---------------------------|\n",
    );
    for permission in Permission::ALL {
        out.push_str(&row(&[
            format!("`{}`", permission.code()),
            text(permission.topic().name()),
            permission.counts().code().to_owned(),
            capital(permission.kind().code()),
            text(permission.sentence()),
        ]));
    }
    out
}

/// The permissions of the starting roles, and whether each asks for two-step sign-in.
fn starting_permissions() -> String {
    let mut out = role_head("Permission", ":-:");
    for permission in Permission::ALL {
        let mut cells = vec![text(permission.sentence())];
        cells.extend(StartingRole::ALL.iter().map(|role| {
            if role.permissions().contains(permission) {
                "✓".to_owned()
            } else {
                "–".to_owned()
            }
        }));
        out.push_str(&row(&cells));
    }
    let mut cells = vec!["Two-step sign-in for everyone with this role".to_owned()];
    cells.extend(
        StartingRole::ALL
            .iter()
            .map(|role| if role.two_step() { "on" } else { "off" }.to_owned()),
    );
    out.push_str(&row(&cells));
    out
}

/// The letter that stands for a field level in the table.
fn field_letter(level: FieldLevel) -> char {
    match level {
        FieldLevel::CanChange => 'C',
        FieldLevel::CanSee => 'S',
        FieldLevel::Hidden => 'H',
    }
}

/// The letter that stands for a *Who sees people* level in the table.
fn people_letter(level: PeopleLevel) -> char {
    match level {
        PeopleLevel::Names => 'N',
        PeopleLevel::RoleOnly => 'R',
        PeopleLevel::Hidden => 'H',
    }
}

/// What each letter of a table stands for.
fn legend<L: bb_core::words::Word>(letter: fn(L) -> char, name: fn(L) -> MessageKey) -> String {
    let parts: Vec<String> = L::ALL
        .iter()
        .rev()
        .map(|level| format!("{} = *{}*", letter(*level), text(name(*level))))
        .collect();
    format!("{}.\n\n", parts.join(", "))
}

/// The starting levels of the fields.
fn starting_field_levels() -> String {
    let mut out = legend(field_letter, FieldLevel::name);
    out.push_str(&role_head("Field", ":-:"));
    for field in Field::ALL {
        let mut cells = vec![text(field.name())];
        cells.extend(
            StartingRole::ALL
                .iter()
                .map(|role| field_letter(role.field_level(*field)).to_string()),
        );
        out.push_str(&row(&cells));
    }
    out
}

/// The starting *Who sees people* levels.
fn starting_people_levels() -> String {
    let mut out = legend(people_letter, PeopleLevel::name);
    out.push_str(&role_head("Kind of people information", ":-:"));
    for kind in PeopleKind::ALL {
        let mut cells = vec![text(kind.name())];
        cells.extend(
            StartingRole::ALL
                .iter()
                .map(|role| people_letter(role.people_level(*kind)).to_string()),
        );
        out.push_str(&row(&cells));
    }
    out
}

/// Every generated table of the page: its name in the comments, and its text.
fn tables() -> [(&'static str, String); 5] {
    [
        ("field-catalogue", field_catalogue()),
        ("permission-catalogue", permission_catalogue()),
        ("starting-permissions", starting_permissions()),
        ("starting-field-levels", starting_field_levels()),
        ("starting-people-levels", starting_people_levels()),
    ]
}

/// The page with the text between the comments of the table `name` replaced by `table`.
fn replace_table(page: &str, name: &str, table: &str) -> anyhow::Result<String> {
    let begin = format!("{BEGIN}{name} {NOTE}\n");
    let end = format!("{END}{name} -->\n");
    let start = page
        .find(&format!("{BEGIN}{name} "))
        .with_context(|| format!("{PAGE} has no `{BEGIN}{name} …` comment"))?;
    let line_end = page[start..]
        .find('\n')
        .map(|offset| start + offset + 1)
        .with_context(|| format!("{PAGE}: the comment of `{name}` is cut off"))?;
    let Some(stop) = page[line_end..].find(&end).map(|offset| line_end + offset) else {
        bail!("{PAGE} has no `{END}{name} -->` comment after the first one");
    };
    Ok(format!(
        "{}{begin}\n{table}\n\n{}",
        &page[..start],
        &page[stop..]
    ))
}

/// The page, with every table as the code says.
pub(crate) fn generate(repo: &Repo) -> anyhow::Result<Vec<Generated>> {
    let mut page = read_text(repo, PAGE)?;
    for (name, table) in tables() {
        page = replace_table(&page, name, table.trim_end_matches('\n'))?;
    }
    Ok(vec![Generated::new(PAGE, page)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn good_example_the_table_between_the_comments_is_replaced_and_the_rest_stays() {
        let page = format!("before\n{BEGIN}t {NOTE}\nold\n{END}t -->\nafter\n");
        let made = replace_table(&page, "t", "| new |").unwrap();
        assert_eq!(
            made,
            format!("before\n{BEGIN}t {NOTE}\n\n| new |\n\n{END}t -->\nafter\n")
        );
        // A second run changes nothing.
        assert_eq!(replace_table(&made, "t", "| new |").unwrap(), made);
    }

    #[test]
    fn bad_example_a_page_without_the_comments_is_an_error() {
        assert!(replace_table("no comments\n", "t", "x").is_err());
        let cut = format!("{BEGIN}t {NOTE}\nold\n");
        assert!(replace_table(&cut, "t", "x").is_err());
    }

    #[test]
    fn the_tables_have_a_row_for_every_permission_field_and_kind() {
        let counted = |table: &str| table.lines().filter(|line| line.starts_with("| ")).count();
        assert_eq!(counted(&field_catalogue()), Field::ALL.len() + 1);
        assert_eq!(counted(&permission_catalogue()), Permission::ALL.len() + 1);
        assert_eq!(counted(&starting_permissions()), Permission::ALL.len() + 2);
        assert_eq!(counted(&starting_field_levels()), Field::ALL.len() + 1);
        assert_eq!(
            counted(&starting_people_levels()),
            PeopleKind::ALL.len() + 1
        );
    }

    #[test]
    fn the_legend_names_every_letter() {
        assert_eq!(
            legend(field_letter, FieldLevel::name),
            "C = *Can change*, S = *Can see*, H = *Hidden*.\n\n"
        );
    }

    #[test]
    fn the_page_of_this_repository_has_all_the_tables() {
        let repo = Repo::discover().unwrap();
        assert_eq!(generate(&repo).unwrap().len(), 1);
    }
}
