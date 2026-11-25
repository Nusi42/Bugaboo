//! The columns of a board (M10-Q1): the statuses of the workflows of the issues found, those of the
//! same name merged (`same_name`) in the order of the workflow of the first project; the three
//! status groups; or the choices of an own field that holds one value, and *None*. Each column is a
//! value of the field of the columns, and the store places each issue by it with the reader's
//! rights.

use bb_core::ErrorCode;
use bb_core::fields::code_phrase;
use bb_core::id::{FieldId, IssueTypeId};
use bb_core::issue::StatusGroup;
use bb_core::names::{name_key, same_name};
use bb_core::own_fields::FieldType;
use bb_core::saved_search::BoardOptions;
use bb_core::workflow::WorkflowContent;
use bb_searchlang::{Expr, FieldRef, Operand, Span, Test};

use crate::auth::AppError;
use crate::search::{FieldTarget, GroupKey, Looked, target_of};

/// What the columns of a board come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ColumnsBy {
    /// The statuses, merged by name across workflows.
    Status,
    /// The three status groups.
    StatusGroup,
    /// The choices of an own field that holds one value, and *None*.
    Choice(FieldId),
}

/// One column, as the board reads it.
#[derive(Clone, Debug)]
pub(crate) struct Column {
    /// What it stands for, as the search language writes it; none for *None*.
    pub(crate) key: Option<String>,
    /// The status group of a column of a status group.
    pub(crate) status_group: Option<StatusGroup>,
    /// The values of the field of the columns that fall into it.
    pub(crate) groups: Vec<GroupKey>,
}

/// The search word of the field the columns come from: the one the options name, or *Status*.
pub(crate) fn columns_word(options: &BoardOptions) -> String {
    options
        .columns
        .clone()
        .unwrap_or_else(|| bb_core::fields::Field::Status.search_word())
}

/// What the columns of `options` come from, for a reader who may search by `looked`; `None` for a
/// field the reader may not read or that cannot be the columns of a board.
pub(crate) fn columns_by(looked: &Looked, options: &BoardOptions) -> Option<ColumnsBy> {
    match target_of(&looked.fields, &columns_word(options))? {
        FieldTarget::Status => Some(ColumnsBy::Status),
        FieldTarget::StatusGroup => Some(ColumnsBy::StatusGroup),
        FieldTarget::Own {
            field,
            kind: FieldType::PickOne,
        } => Some(ColumnsBy::Choice(field)),
        _ => None,
    }
}

/// `<word> <test>`, a filter of the language.
pub(crate) fn field_test(word: &str, test: Test) -> Expr {
    let span = Span::default();
    Expr::Field {
        field: FieldRef {
            word: word.to_owned(),
            span,
        },
        test,
        span,
    }
}

/// The values the card limits of `options` name, as one search (`<columns> in (…)`), so the names
/// they hold are checked by the one resolution of names of the search (ADR-0034, amendment of
/// 2026-11-24): a limit that names a status or a choice the reader cannot know makes the saved
/// search not check for them. None when no limit names a value.
pub(crate) fn named_by_limits(options: &BoardOptions) -> Option<bb_searchlang::Search> {
    let span = Span::default();
    let values: Vec<Operand> = options
        .limits
        .iter()
        .filter_map(|limit| limit.column.clone())
        .map(|text| Operand::Text { text, span })
        .collect();
    (!values.is_empty()).then(|| bb_searchlang::Search {
        filter: Some(field_test(
            &columns_word(options),
            Test::In { values, span },
        )),
        sort: Vec::new(),
        span,
    })
}

/// The columns of a board for the reader. Of statuses: the statuses of the workflows in force for
/// the issue types `found` where the search looks (all types when nothing is found), in the order
/// of the first project's workflow; a status joins a column of the same name, or the column its key
/// is in already, so a status key stands in one column only and an issue is counted once (a status
/// key that two workflows name differently is in the column of its first name). Empty columns stay.
///
/// # Errors
/// `request.invalid` for a field of choices that is not one the reader may know.
pub(crate) fn board_columns(
    looked: &Looked,
    by: ColumnsBy,
    found: &[IssueTypeId],
) -> Result<Vec<Column>, AppError> {
    Ok(match by {
        ColumnsBy::Status => status_columns(
            looked
                .workflows
                .iter()
                .filter(|(issue_type, _)| found.is_empty() || found.contains(issue_type))
                .map(|(_, workflow)| &**workflow),
        ),
        ColumnsBy::StatusGroup => StatusGroup::ALL
            .iter()
            .map(|group| Column {
                key: Some(code_phrase(group.code())),
                status_group: Some(*group),
                groups: vec![GroupKey::StatusGroup(*group)],
            })
            .collect(),
        ColumnsBy::Choice(field) => {
            let definition = looked
                .own
                .iter()
                .find(|definition| definition.field.id == field)
                .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
            definition
                .choices
                .iter()
                .filter(|choice| choice.retired_at.is_none())
                .map(|choice| Column {
                    key: Some(choice.name.clone()),
                    status_group: None,
                    groups: vec![GroupKey::Choice(choice.id)],
                })
                .chain(std::iter::once(Column {
                    key: None,
                    status_group: None,
                    groups: vec![GroupKey::Empty],
                }))
                .collect()
        }
    })
}

/// The columns of statuses of these workflows, in their order: a status stands in the column its
/// key is in already, else in the column of its name, else in a column of its own. So a status key
/// is in one column only (a key that two workflows name differently stays in the column of its
/// first name) and an issue is counted once.
fn status_columns<'a>(workflows: impl Iterator<Item = &'a WorkflowContent>) -> Vec<Column> {
    let mut columns: Vec<Column> = Vec::new();
    for workflow in workflows {
        for status in &workflow.statuses {
            let key = GroupKey::Text(status.key.to_string());
            if columns.iter().any(|column| column.groups.contains(&key)) {
                continue;
            }
            match columns.iter_mut().find(|column| {
                column
                    .key
                    .as_deref()
                    .is_some_and(|name| same_name(name, &status.name))
            }) {
                Some(column) => column.groups.push(key),
                None => columns.push(Column {
                    key: Some(status.name.clone()),
                    status_group: None,
                    groups: vec![key],
                }),
            }
        }
    }
    columns
}

/// The column of each value of the field of the columns, for the store.
pub(crate) fn column_of(columns: &[Column]) -> Vec<(GroupKey, usize)> {
    columns
        .iter()
        .enumerate()
        .flat_map(|(at, column)| column.groups.iter().map(move |key| (key.clone(), at)))
        .collect()
}

/// Whether two column keys are the same column (the card limit of a status names it by any
/// spelling of its name).
pub(crate) fn same_column(one: Option<&str>, other: Option<&str>) -> bool {
    match (one, other) {
        (Some(one), Some(other)) => name_key(one) == name_key(other),
        (None, None) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use bb_core::issue::StatusGroup;
    use bb_core::workflow::{StatusKey, WorkflowContent, WorkflowStatus};

    use super::{column_of, status_columns};
    use crate::search::GroupKey;

    fn workflow(statuses: &[(&str, &str)]) -> WorkflowContent {
        WorkflowContent {
            statuses: statuses
                .iter()
                .map(|(key, name)| WorkflowStatus {
                    key: StatusKey::new(key).unwrap(),
                    name: (*name).to_owned(),
                    group: StatusGroup::Done,
                })
                .collect(),
            changes: Vec::new(),
        }
    }

    #[test]
    fn plan_06_a_status_key_stands_in_one_column_whatever_the_names_of_other_workflows() {
        // A has K "Done" and M "Closed"; B calls M "Done".
        let first = workflow(&[("k", "Done"), ("m", "Closed")]);
        let second = workflow(&[("m", "Done")]);
        let columns = status_columns([&first, &second].into_iter());
        let names: Vec<Option<&str>> = columns.iter().map(|each| each.key.as_deref()).collect();
        assert_eq!(names, [Some("Done"), Some("Closed")]);
        let m = GroupKey::Text("m".to_owned());
        let placed: Vec<usize> = column_of(&columns)
            .into_iter()
            .filter(|(key, _)| *key == m)
            .map(|(_, at)| at)
            .collect();
        // M is in the column of its first name only, so the heads and the cells count it once.
        assert_eq!(placed, [1]);
        // Statuses of the same name and of different keys are one column.
        let third = workflow(&[("done", "Done")]);
        let merged = status_columns([&first, &third].into_iter());
        assert_eq!(
            merged[0].groups,
            [
                GroupKey::Text("k".to_owned()),
                GroupKey::Text("done".to_owned())
            ]
        );
    }
}
