//! The levels roles have set, in the six tables (field levels and people levels, each for a
//! role's own level and for a project; field levels also for one issue type, ADR-0027): read and
//! written once, over [`LevelSubject`], so the kinds share every statement (ADR-0025,
//! decisions 10 and 11).

use bb_core::access::FieldLevel;
use bb_core::fields::FieldKey;
use bb_core::id::{IssueTypeId, ProjectId, RoleId};
use bb_core::words::Word;
use bb_policy::{LevelKind, LevelRule, LevelSubject, TypedFieldRule};
use sea_query::{Expr, ExprTrait as _, IdenStatic as _, Query};

use super::delete_by;
use crate::db::{Reader, StoredId as _, Transaction};
use crate::error::StoreError;
use crate::schema::{
    ProjectFieldLevelRow, ProjectPeopleLevelRow, ProjectTypeFieldLevelRow, RoleFieldLevelRow,
    RolePeopleLevelRow, RoleTypeFieldLevelRow,
};

/// One of the six tables: its name and the names of its columns.
#[derive(Clone, Copy)]
struct Table {
    name: &'static str,
    /// The column of the project, for the tables of levels set for a project.
    project: Option<&'static str>,
    /// The column of the issue type, for the tables of levels set for one issue type.
    issue_type: Option<&'static str>,
    role: &'static str,
    subject: &'static str,
    level: &'static str,
    /// What a subject, and a level, that cannot be read are, for the report of a corrupt row.
    subject_is: &'static str,
    level_is: &'static str,
}

/// The table of one kind of level: a role's own (`in_project` is false) or set for a project,
/// for all issue types or for one (`typed`; only fields have those).
fn table(kind: LevelKind, in_project: bool, typed: bool) -> Result<Table, StoreError> {
    Ok(match (kind, in_project, typed) {
        (LevelKind::Field, false, false) => Table {
            name: RoleFieldLevelRow::Table.as_str(),
            project: None,
            issue_type: None,
            role: RoleFieldLevelRow::Role.as_str(),
            subject: RoleFieldLevelRow::Field.as_str(),
            level: RoleFieldLevelRow::Level.as_str(),
            subject_is: "a field key",
            level_is: "a field level",
        },
        (LevelKind::Field, true, false) => Table {
            name: ProjectFieldLevelRow::Table.as_str(),
            project: Some(ProjectFieldLevelRow::Project.as_str()),
            issue_type: None,
            role: ProjectFieldLevelRow::Role.as_str(),
            subject: ProjectFieldLevelRow::Field.as_str(),
            level: ProjectFieldLevelRow::Level.as_str(),
            subject_is: "a field key",
            level_is: "a field level",
        },
        (LevelKind::Field, false, true) => Table {
            name: RoleTypeFieldLevelRow::Table.as_str(),
            project: None,
            issue_type: Some(RoleTypeFieldLevelRow::IssueType.as_str()),
            role: RoleTypeFieldLevelRow::Role.as_str(),
            subject: RoleTypeFieldLevelRow::Field.as_str(),
            level: RoleTypeFieldLevelRow::Level.as_str(),
            subject_is: "a field key",
            level_is: "a field level",
        },
        (LevelKind::Field, true, true) => Table {
            name: ProjectTypeFieldLevelRow::Table.as_str(),
            project: Some(ProjectTypeFieldLevelRow::Project.as_str()),
            issue_type: Some(ProjectTypeFieldLevelRow::IssueType.as_str()),
            role: ProjectTypeFieldLevelRow::Role.as_str(),
            subject: ProjectTypeFieldLevelRow::Field.as_str(),
            level: ProjectTypeFieldLevelRow::Level.as_str(),
            subject_is: "a field key",
            level_is: "a field level",
        },
        (LevelKind::People, false, false) => Table {
            name: RolePeopleLevelRow::Table.as_str(),
            project: None,
            issue_type: None,
            role: RolePeopleLevelRow::Role.as_str(),
            subject: RolePeopleLevelRow::Kind.as_str(),
            level: RolePeopleLevelRow::Level.as_str(),
            subject_is: "a kind of people information",
            level_is: "a people level",
        },
        (LevelKind::People, true, false) => Table {
            name: ProjectPeopleLevelRow::Table.as_str(),
            project: Some(ProjectPeopleLevelRow::Project.as_str()),
            issue_type: None,
            role: ProjectPeopleLevelRow::Role.as_str(),
            subject: ProjectPeopleLevelRow::Kind.as_str(),
            level: ProjectPeopleLevelRow::Level.as_str(),
            subject_is: "a kind of people information",
            level_is: "a people level",
        },
        // Only fields have levels per issue type.
        (LevelKind::People, _, true) => {
            return Err(StoreError::Corrupt(
                "a level of people information for an issue type",
            ));
        }
    })
}

/// Which levels to read.
#[derive(Clone, Copy)]
pub(crate) enum Which<'a> {
    /// Every level of every role.
    All,
    /// Every level these roles have set, for the organization and for all projects.
    Roles(&'a [RoleId]),
    /// The levels of these roles that apply at a place: their own, and those set for a project of
    /// `chain` (the projects from the top down to the place).
    At {
        roles: &'a [RoleId],
        chain: &'a [ProjectId],
    },
}

/// The levels of one kind that roles have set, their own and those set for projects, for all
/// issue types: one statement for both tables.
pub(crate) async fn read_rules<K: LevelSubject>(
    reader: &mut impl Reader,
    which: Which<'_>,
) -> Result<Vec<LevelRule<K>>, StoreError> {
    let tables = [table(K::KIND, false, false)?, table(K::KIND, true, false)?];
    read_rows(reader, tables, which)
        .await?
        .into_iter()
        .map(|row| {
            Ok(LevelRule {
                project: row.project,
                role: row.role,
                subject: K::from_subject_code(&row.subject)
                    .ok_or(StoreError::Corrupt(tables[0].subject_is))?,
                level: <K::Level as Word>::from_code(&row.level)
                    .ok_or(StoreError::Corrupt(tables[0].level_is))?,
            })
        })
        .collect()
}

/// The levels of fields that roles have set for one issue type, their own and those set for
/// projects: one statement for both tables.
pub(crate) async fn read_typed_rules(
    reader: &mut impl Reader,
    which: Which<'_>,
) -> Result<Vec<TypedFieldRule>, StoreError> {
    let tables = [
        table(LevelKind::Field, false, true)?,
        table(LevelKind::Field, true, true)?,
    ];
    read_rows(reader, tables, which)
        .await?
        .into_iter()
        .map(|row| {
            Ok(TypedFieldRule {
                project: row.project,
                role: row.role,
                issue_type: row
                    .issue_type
                    .ok_or(StoreError::Corrupt("an issue type of a level"))?,
                subject: FieldKey::from_code(&row.subject)
                    .ok_or(StoreError::Corrupt(tables[0].subject_is))?,
                level: <FieldLevel as Word>::from_code(&row.level)
                    .ok_or(StoreError::Corrupt(tables[0].level_is))?,
            })
        })
        .collect()
}

/// A row of any of the tables, before its words are read.
struct RawRule {
    project: Option<ProjectId>,
    role: RoleId,
    issue_type: Option<IssueTypeId>,
    subject: String,
    level: String,
}

/// The names the rows of both tables are read under, so the two selects line up in one `UNION ALL`.
const ROLE: &str = "level_role";
const SUBJECT: &str = "level_subject";
const LEVEL: &str = "level_level";
const PROJECT: &str = "level_project";
const ISSUE_TYPE: &str = "level_issue_type";

/// The select of one table, narrowed as `which` says, with its columns under the common names. A
/// table of a role's own levels has no project: its rows say none (a typed NULL).
fn select_of(table: Table, which: Which<'_>) -> sea_query::SelectStatement {
    let mut select = Query::select();
    select
        .expr_as(Expr::col(table.role), sea_query::Alias::new(ROLE))
        .expr_as(Expr::col(table.subject), sea_query::Alias::new(SUBJECT))
        .expr_as(Expr::col(table.level), sea_query::Alias::new(LEVEL))
        .from(table.name);
    match table.project {
        Some(project) => select.expr_as(Expr::col(project), sea_query::Alias::new(PROJECT)),
        None => select.expr_as(Expr::val(None::<Vec<u8>>), sea_query::Alias::new(PROJECT)),
    };
    if let Some(issue_type) = table.issue_type {
        select.expr_as(Expr::col(issue_type), sea_query::Alias::new(ISSUE_TYPE));
    }
    if let (Which::At { chain, .. }, Some(project)) = (which, table.project) {
        select.and_where(Expr::col(project).is_in(super::in_list(chain)));
    }
    if let Which::Roles(roles) | Which::At { roles, .. } = which {
        select.and_where(Expr::col(table.role).is_in(super::in_list(roles)));
    }
    select
}

/// The rows of a role's own table and of the table set for projects, in one statement. At the
/// organization (an empty chain) nothing is set for a project, so only the first is read.
async fn read_rows(
    reader: &mut impl Reader,
    tables: [Table; 2],
    which: Which<'_>,
) -> Result<Vec<RawRule>, StoreError> {
    let mut select = select_of(tables[0], which);
    let organization_only = matches!(which, Which::At { chain, .. } if chain.is_empty());
    if !organization_only {
        select.union(sea_query::UnionType::All, select_of(tables[1], which));
    }
    let mut rows = Vec::new();
    for row in reader.fetch_all(&select).await? {
        rows.push(RawRule {
            project: row.optional_id::<ProjectId>(PROJECT)?,
            role: row.id::<RoleId>(ROLE)?,
            issue_type: if tables[0].issue_type.is_some() {
                Some(row.id::<IssueTypeId>(ISSUE_TYPE)?)
            } else {
                None
            },
            subject: row.get::<String>(SUBJECT)?,
            level: row.get::<String>(LEVEL)?,
        });
    }
    Ok(rows)
}

/// The levels a role has set for the subjects of one kind: in a project (`Some`) or its own, for
/// all issue types or for one.
pub(crate) async fn read_levels<K: LevelSubject>(
    reader: &mut impl Reader,
    role: RoleId,
    project: Option<ProjectId>,
    issue_type: Option<IssueTypeId>,
) -> Result<Vec<(K, K::Level)>, StoreError> {
    let table = table(K::KIND, project.is_some(), issue_type.is_some())?;
    let mut select = Query::select();
    select
        .columns([table.subject, table.level])
        .from(table.name)
        .and_where(Expr::col(table.role).eq(role.stored()));
    if let (Some(column), Some(project)) = (table.project, project) {
        select.and_where(Expr::col(column).eq(project.stored()));
    }
    if let (Some(column), Some(issue_type)) = (table.issue_type, issue_type) {
        select.and_where(Expr::col(column).eq(issue_type.stored()));
    }
    reader
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| {
            Ok((
                K::from_subject_code(&row.get::<String>(table.subject)?)
                    .ok_or(StoreError::Corrupt(table.subject_is))?,
                <K::Level as Word>::from_code(&row.get::<String>(table.level)?)
                    .ok_or(StoreError::Corrupt(table.level_is))?,
            ))
        })
        .collect()
}

/// Adds the level of a subject for a role.
pub(crate) async fn insert_level<K: LevelSubject>(
    transaction: &mut Transaction,
    role: RoleId,
    project: Option<ProjectId>,
    issue_type: Option<IssueTypeId>,
    subject: K,
    level: K::Level,
) -> Result<(), StoreError> {
    let table = table(K::KIND, project.is_some(), issue_type.is_some())?;
    let mut columns = vec![table.role, table.subject, table.level];
    let mut values = vec![
        Expr::val(role.stored()),
        Expr::val(subject.subject_code().into_owned()),
        Expr::val(level.code()),
    ];
    if let (Some(column), Some(project)) = (table.project, project) {
        columns.push(column);
        values.push(Expr::val(project.stored()));
    }
    if let (Some(column), Some(issue_type)) = (table.issue_type, issue_type) {
        columns.push(column);
        values.push(Expr::val(issue_type.stored()));
    }
    let mut insert = Query::insert();
    insert
        .into_table(table.name)
        .columns(columns)
        .values(values)
        .map_err(|_| StoreError::Corrupt(table.level_is))?;
    transaction.execute(&insert).await.map(|_| ())
}

/// Sets (`Some`) or takes away (`None`) the level of a subject for a role, in a project or the
/// role's own, for all issue types or for one.
pub(crate) async fn put_level<K: LevelSubject>(
    transaction: &mut Transaction,
    role: RoleId,
    project: Option<ProjectId>,
    issue_type: Option<IssueTypeId>,
    subject: K,
    level: Option<K::Level>,
) -> Result<(), StoreError> {
    let table = table(K::KIND, project.is_some(), issue_type.is_some())?;
    let mut delete = Query::delete();
    delete
        .from_table(table.name)
        .and_where(Expr::col(table.role).eq(role.stored()))
        .and_where(Expr::col(table.subject).eq(subject.subject_code().into_owned()));
    if let (Some(column), Some(project)) = (table.project, project) {
        delete.and_where(Expr::col(column).eq(project.stored()));
    }
    if let (Some(column), Some(issue_type)) = (table.issue_type, issue_type) {
        delete.and_where(Expr::col(column).eq(issue_type.stored()));
    }
    transaction.execute(&delete).await?;
    match level {
        Some(level) => insert_level(transaction, role, project, issue_type, subject, level).await,
        None => Ok(()),
    }
}

/// Adds the levels a role has of its own for the subjects of one kind.
pub(crate) async fn insert_own<K: LevelSubject>(
    transaction: &mut Transaction,
    role: RoleId,
    levels: &[(K, K::Level)],
) -> Result<(), StoreError> {
    for (subject, level) in levels {
        insert_level(transaction, role, None, None, *subject, *level).await?;
    }
    Ok(())
}

/// Replaces the levels a role has of its own for the subjects of one kind (not those set for
/// projects).
pub(crate) async fn replace_own<K: LevelSubject>(
    transaction: &mut Transaction,
    role: RoleId,
    levels: &[(K, K::Level)],
) -> Result<(), StoreError> {
    let own = table(K::KIND, false, false)?;
    delete_by(transaction, own.name, own.role, &role.stored()).await?;
    insert_own(transaction, role, levels).await
}

/// Removes every level a role has set, of both kinds, its own and those set for projects, for all
/// issue types and for one.
pub(crate) async fn delete_all_of(
    transaction: &mut Transaction,
    role: RoleId,
) -> Result<(), StoreError> {
    for kind in LevelKind::ALL {
        for in_project in [false, true] {
            for typed in [false, true] {
                // People information has no levels per issue type.
                let Ok(table) = table(*kind, in_project, typed) else {
                    continue;
                };
                delete_by(transaction, table.name, table.role, &role.stored()).await?;
            }
        }
    }
    Ok(())
}
