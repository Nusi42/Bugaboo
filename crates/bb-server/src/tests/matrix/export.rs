//! The file of *Download my data* in the permission matrix (SEC-MATRIX, PRIV-03, M2-F65, ADR-0044,
//! decision 3): for every starting role, a person makes the file of their own data, and the record
//! has a line for every kind of record in it that says what of the masked fields and of other
//! people the file held. The entry point is `export`; its lines in
//! `tools/checks/sec-matrix-record.txt` start with `export`.
//!
//! The scene: the person reports an issue where they may, and the administrator then gives it a
//! status and an estimate, so the person's file holds an issue whose fields some roles may not see,
//! and the notification of that change, which names the administrator. A field hidden from a role
//! on screen must be hidden in the file it makes of itself, and nobody may be shown more openly than
//! the strictest of the role's levels of *Who sees people*.

use std::collections::BTreeMap;

use bb_api_words::sec_matrix as words;
use bb_app::access::StartingRole;
use bb_app::privacy::export::field_of;
use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::id::UserId;
use bb_core::people_kinds::PeopleKind;
use bb_testing::{Failure, TestDatabase};
use serde_json::{Value, json};

use super::super::data_exports_http::{MyFile, my_file};
use super::record::matches_the_repository_for;
use crate::sign_in_tests::http::rig_and_app;
use crate::sign_in_tests::issues_site::{ISSUES, Site, key_of, ok, version_of};

/// The kind of record of issues in the file, and the names its JSON gives the two fields the lines
/// say about: *Status*, which every role that sees an issue sees, and *Estimated time*, which some
/// roles may not see.
const ISSUE_KIND: &str = "issue";
const STATUS_KEY: &str = "status";
const ESTIMATE_KEY: &str = "estimated_time";
/// What the administrator sets the estimate to, in minutes.
const ESTIMATE_MINUTES: i64 = 240;

/// What one kind of record of one person's file held.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Exported {
    pub(super) role: StartingRole,
    /// The kind of record: the name of its data file.
    pub(super) kind: String,
    /// Whether the status of an issue was in it.
    pub(super) status: &'static str,
    /// Whether the estimate of an issue was in it.
    pub(super) estimate: &'static str,
    /// The most open way another person was shown in it; `None` when it names nobody else.
    pub(super) others: Option<PeopleLevel>,
}

impl Exported {
    /// The line of the record.
    pub(super) fn line(&self) -> String {
        format!(
            "{}{} {} : status={} estimate={} others={}",
            words::EXPORT_LINE,
            self.kind,
            words::role_actor(self.role.code()),
            self.status,
            self.estimate,
            self.others.map_or(words::NOT_IN_KIND, PeopleLevel::code),
        )
    }
}

/// The values inside `value`, at every depth: the members of an object and the items of a list,
/// so a person or a field inside the changes of an event is found like one at the top of a row.
fn inside(value: &Value) -> Box<dyn Iterator<Item = &Value> + '_> {
    let children: Box<dyn Iterator<Item = &Value>> = match value {
        Value::Object(members) => Box::new(members.values()),
        Value::Array(items) => Box::new(items.iter()),
        _ => Box::new(std::iter::empty()),
    };
    Box::new(std::iter::once(value).chain(children.flat_map(inside)))
}

/// Whether `value` holds the field `key` with a value: as a member of that name, or as a change
/// of the field, named by its member or by its word in the catalogue (`estimated-time`), with a
/// value before or after.
fn holds_field(value: &Value, key: &str) -> bool {
    let word = field_of(key).map(Field::code);
    inside(value).any(|found| {
        found.get(key).is_some_and(|held| !held.is_null())
            || (found
                .get("field")
                .and_then(Value::as_str)
                .is_some_and(|field| field == key || Some(field) == word)
                && ["before", "after"]
                    .iter()
                    .any(|side| found.get(side).is_some_and(|held| !held.is_null())))
    })
}

/// Whether the rows of a kind hold a field: shown when a row has a value for it anywhere inside,
/// hidden when the kind has the field and no row shows it, none when the kind has no such field at
/// all.
fn field_word(kind: &str, rows: &[Value], key: &str) -> &'static str {
    if rows.iter().any(|row| holds_field(row, key)) {
        words::SHOWN
    } else if kind == ISSUE_KIND {
        words::LEFT_OUT
    } else {
        words::NOT_IN_KIND
    }
}

/// How a person is shown in a value of the file, when the value is a person and not `me`.
fn other_in(value: &Value, me: UserId) -> Option<PeopleLevel> {
    match value.get("shown")?.as_str()? {
        "name" if value.get("id")?.as_str()? == me.to_string() => None,
        "name" => Some(PeopleLevel::Names),
        "role" => Some(PeopleLevel::RoleOnly),
        _ => Some(PeopleLevel::Hidden),
    }
}

/// The most open way the rows show anybody but `me`, at any depth of a row.
fn others_in(rows: &[Value], me: UserId) -> Option<PeopleLevel> {
    rows.iter()
        .flat_map(inside)
        .filter_map(|value| other_in(value, me))
        .max()
}

/// The lines of one person's file: one for each kind of record in it.
fn lines_of(role: StartingRole, me: UserId, file: &mut MyFile) -> Result<Vec<Exported>, Failure> {
    let kinds: Vec<String> = file
        .names()
        .into_iter()
        .filter_map(|name| {
            name.strip_prefix("data/")?
                .strip_suffix(".json")
                .map(str::to_owned)
        })
        .collect();
    let mut found = Vec::new();
    for kind in kinds {
        let rows = file.rows(&kind)?;
        found.push(Exported {
            role,
            status: field_word(&kind, &rows, STATUS_KEY),
            estimate: field_word(&kind, &rows, ESTIMATE_KEY),
            others: others_in(&rows, me),
            kind,
        });
    }
    found.sort_by(|one, other| one.kind.cmp(&other.kind));
    Ok(found)
}

/// What the tables of the starting roles let the role see of a field, in the words of the record.
fn table_word(role: StartingRole, field: Field) -> &'static str {
    if role.field_level(field) == FieldLevel::Hidden {
        words::LEFT_OUT
    } else {
        words::SHOWN
    }
}

/// The strictest level of *Who sees people* the role has: the file shows nobody more openly.
fn strictest(role: StartingRole) -> PeopleLevel {
    PeopleKind::ALL
        .iter()
        .copied()
        .filter(|kind| *kind != PeopleKind::Pictures)
        .map(|kind| role.people_level(kind))
        .min()
        .unwrap_or(PeopleLevel::Hidden)
}

/// Every starting role makes the file of its own data; the lines of every file.
///
/// # Errors
/// The server answers something the scene does not expect.
pub(super) async fn run(db: &TestDatabase) -> Result<Vec<Exported>, Failure> {
    let (rig, app) = rig_and_app(db).await?;
    let (site, mut ada) = Site::start(&rig, &app).await?;
    let mut everyone = Vec::new();
    for role in StartingRole::ALL.iter().copied() {
        let name = format!("Exporter {}", role.code());
        let (mut person, me) = site
            .person_called(role, &format!("exports-{}", role.code()), &name)
            .await?;
        let made = site
            .create(&mut person, &format!("Printer of the {}", role.code()))
            .await?;
        // A role that may not report an issue has none in its file.
        if made.status.is_success() {
            let issue = key_of(&made)?;
            let version = version_of(&mut ada, &issue).await?;
            ok(&ada
                .send(
                    "PATCH",
                    &format!("{ISSUES}/{issue}"),
                    Some(json!({
                        "version": version, "status": "done",
                        "estimated_time": ESTIMATE_MINUTES,
                    })),
                )
                .await?)?;
        }
        let mut file = my_file(&rig, &mut person).await?;
        everyone.extend(lines_of(role, me, &mut file)?);
    }
    Ok(everyone)
}

#[tokio::test]
async fn sec_01_the_file_a_person_makes_of_themselves_shows_no_more_than_their_screens()
-> bb_testing::TestResult {
    bb_testing::each_database("sec_matrix_export", async |db| {
        let exported = run(db).await?;
        let mut issues: BTreeMap<StartingRole, &Exported> = BTreeMap::new();
        for each in &exported {
            if each.kind == ISSUE_KIND {
                issues.insert(each.role, each);
                assert_eq!(
                    each.status,
                    table_word(each.role, Field::Status),
                    "{}",
                    each.line()
                );
                assert_eq!(
                    each.estimate,
                    table_word(each.role, Field::EstimatedTime),
                    "{}",
                    each.line()
                );
            }
            if let Some(others) = each.others {
                assert!(others <= strictest(each.role), "{}", each.line());
            }
        }
        // Every role that may report an issue has it in its file, so the masked fields are in play.
        assert!(!issues.is_empty());
        let lines: Vec<String> = exported.iter().map(Exported::line).collect();
        matches_the_repository_for(words::EXPORT_LINE, &lines)
    })
    .await
}
