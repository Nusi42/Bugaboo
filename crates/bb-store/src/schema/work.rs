//! The tables of projects, issues, own fields, relations, layouts, labels, tags and the runs of changes to many issues.

use super::{Iden, IdenStatic};

/// Projects (PRJ-01): the tree that roles are given on.
#[derive(Clone, Copy, Iden)]
pub(crate) enum ProjectRow {
    #[iden = "project"]
    Table,
    Id,
    Parent,
    ShortName,
    Name,
    Version,
}

/// The old short names of projects: each stays with its project (PRJ-01, M2-Q4).
#[derive(Clone, Copy, Iden)]
pub(crate) enum ProjectAliasRow {
    #[iden = "project_alias"]
    Table,
    ShortName,
    Project,
    RetiredAt,
}

/// Issue types (ISS-02).
#[derive(Clone, Copy, Iden)]
pub(crate) enum IssueTypeRow {
    #[iden = "issue_type"]
    Table,
    Id,
    Name,
    Icon,
    Position,
    RetiredAt,
    Version,
}

/// Issues (ISS-01). The code is the part of the readable ID after the project's short name.
#[derive(Clone, Copy, Iden)]
pub(crate) enum IssueRow {
    #[iden = "issue"]
    Table,
    Id,
    Project,
    IssueType,
    Code,
    Title,
    Description,
    Status,
    Priority,
    ReportedBy,
    CreatedAt,
    UpdatedAt,
    ClosedAt,
    DeletedAt,
    Version,
}

/// Own fields (CF-01).
#[derive(Clone, Copy, Iden)]
pub(crate) enum FieldRow {
    #[iden = "field"]
    Table,
    Id,
    Name,
    Help,
    Kind,
    AppliesTo,
    Rules,
    DefaultValue,
    Look,
    NeverToAssistant,
    Position,
    RetiredAt,
    Version,
}

/// The projects an own field is limited to.
#[derive(Clone, Copy, Iden)]
pub(crate) enum FieldProjectRow {
    #[iden = "field_project"]
    Table,
    Field,
    Project,
}

/// The issue types an own field is limited to.
#[derive(Clone, Copy, Iden)]
pub(crate) enum FieldIssueTypeRow {
    #[iden = "field_issue_type"]
    Table,
    Field,
    IssueType,
}

/// The choices of own fields (CF-04).
#[derive(Clone, Copy, Iden)]
pub(crate) enum FieldChoiceRow {
    #[iden = "field_option"]
    Table,
    Id,
    Field,
    Name,
    Color,
    Icon,
    Position,
    RetiredAt,
}

/// The values of own fields (ADR-0006): one row per value, one typed column per kind.
#[derive(Clone, Copy, Iden)]
pub(crate) enum FieldValueRow {
    #[iden = "field_value"]
    Table,
    EntityKind,
    EntityId,
    Field,
    ValueText,
    ValueNumber,
    ValueDate,
    ValueRef,
    Position,
}

/// The level of a field for a role and one issue type, for the whole organization (ADR-0027).
#[derive(Clone, Copy, IdenStatic)]
pub(crate) enum RoleTypeFieldLevelRow {
    #[iden = "role_type_field_level"]
    Table,
    Role,
    IssueType,
    Field,
    Level,
}

/// The level of a field for a role and one issue type, set for a project (ADR-0027).
#[derive(Clone, Copy, IdenStatic)]
pub(crate) enum ProjectTypeFieldLevelRow {
    #[iden = "project_type_field_level"]
    Table,
    Project,
    Role,
    IssueType,
    Field,
    Level,
}

/// The people an issue is assigned to.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AssignedPersonRow {
    #[iden = "assigned_person"]
    Table,
    Issue,
    Person,
    AddedAt,
}

/// The old codes of issues, which keep opening them (ISS-01a, ADR-0021).
#[derive(Clone, Copy, Iden)]
pub(crate) enum IssueAliasRow {
    #[iden = "issue_alias"]
    Table,
    Project,
    Code,
    Issue,
    RetiredAt,
}

/// The last counting code each project gave out (ISS-01a, ADR-0021).
#[derive(Clone, Copy, Iden)]
pub(crate) enum IssueCounterRow {
    #[iden = "issue_counter"]
    Table,
    Project,
    Last,
}

/// Kinds of relation (ISS-04).
#[derive(Clone, Copy, Iden)]
pub(crate) enum RelationKindRow {
    #[iden = "relation_kind"]
    Table,
    Id,
    Forward,
    Backward,
    Position,
    RetiredAt,
    Version,
}

/// The layouts of the issue page and the new-issue form: where each is set (CFG-01, ADR-0027).
#[derive(Clone, Copy, Iden)]
pub(crate) enum LayoutRow {
    #[iden = "layout"]
    Table,
    Id,
    Project,
    IssueType,
    Kind,
    Version,
}

/// A section of a layout.
#[derive(Clone, Copy, Iden)]
pub(crate) enum LayoutSectionRow {
    #[iden = "layout_section"]
    Table,
    Layout,
    Position,
    Tab,
    Title,
}

/// A field in a section of a layout.
#[derive(Clone, Copy, Iden)]
pub(crate) enum LayoutFieldRow {
    #[iden = "layout_field"]
    Table,
    Layout,
    Field,
    Section,
    Position,
}

/// A role a field of a layout is shown to.
#[derive(Clone, Copy, Iden)]
pub(crate) enum LayoutFieldRoleRow {
    #[iden = "layout_field_role"]
    Table,
    Layout,
    Field,
    Role,
}

/// Relations between two issues, stored once from the issue that reads the forward name (ISS-04).
#[derive(Clone, Copy, Iden)]
pub(crate) enum RelationRow {
    #[iden = "relation"]
    Table,
    Id,
    Kind,
    FromIssue,
    ToIssue,
    AddedBy,
    AddedAt,
}

/// The issue another issue sits under (ISS-03).
#[derive(Clone, Copy, Iden)]
pub(crate) enum IssueParentRow {
    #[iden = "issue_parent"]
    Table,
    Child,
    Parent,
    SetBy,
    SetAt,
}

/// Tags (TAG-01). `name_key` is the name in lower case: two names with one key are one tag.
#[derive(Clone, Copy, Iden)]
pub(crate) enum TagRow {
    #[iden = "tag"]
    Table,
    Id,
    Name,
    NameKey,
    Color,
    CreatedAt,
    Version,
}

/// The projects a tag is limited to (TAG-01).
#[derive(Clone, Copy, Iden)]
pub(crate) enum TagProjectRow {
    #[iden = "tag_project"]
    Table,
    Tag,
    Project,
}

/// The tags on issues (TAG-01).
#[derive(Clone, Copy, Iden)]
pub(crate) enum IssueTagRow {
    #[iden = "issue_tag"]
    Table,
    Issue,
    Tag,
    AddedBy,
    AddedAt,
}

/// Groups of labels (TAG-02).
#[derive(Clone, Copy, Iden)]
pub(crate) enum LabelGroupRow {
    #[iden = "label_group"]
    Table,
    Id,
    Name,
    NameKey,
    PickOne,
    Position,
    RetiredAt,
    Version,
}

/// The organization's labels (TAG-02). `group_key` is the ID of the group, or empty without one,
/// so a name is unique inside a group.
#[derive(Clone, Copy, Iden)]
pub(crate) enum LabelRow {
    #[iden = "label"]
    Table,
    Id,
    #[iden = "group_id"]
    Group,
    GroupKey,
    Name,
    NameKey,
    Color,
    Position,
    RetiredAt,
    Version,
}

/// The labels on issues (TAG-02).
#[derive(Clone, Copy, Iden)]
pub(crate) enum IssueLabelRow {
    #[iden = "issue_label"]
    Table,
    Issue,
    Label,
    AddedBy,
    AddedAt,
}

/// Runs of changes to many issues that go on in the background (ISS-09).
#[derive(Clone, Copy, Iden)]
pub(crate) enum ManyChangeRunRow {
    #[iden = "bulk_run"]
    Table,
    Id,
    Person,
    Action,
    Items,
    Status,
    Done,
    Changed,
    Skipped,
    CreatedAt,
    FinishedAt,
}
