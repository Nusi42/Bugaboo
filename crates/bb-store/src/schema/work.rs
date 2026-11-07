//! The tables of projects, issues, own fields, relations, layouts, labels, tags and the runs of changes to many issues.

use super::{Iden, IdenStatic, StoredTable};

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
    /// The project the type was made in; none for the whole organization.
    Project,
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
    EstimatedTime,
    TimeLeft,
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
    Number,
    Color,
    Icon,
    Position,
    RetiredAt,
}

/// The values of own fields (ADR-0006): one row per value, one typed column per kind. The values
/// of time entries have the same columns in `time_entry_value` ([`TimeEntryValueRow`](super::TimeEntryValueRow)).
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

/// A field that was taken off the form of a layout; its roles are rows of `layout_field_role`.
#[derive(Clone, Copy, Iden)]
pub(crate) enum LayoutOffFieldRow {
    #[iden = "layout_off_field"]
    Table,
    Layout,
    Field,
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

/// A workflow: its name and version; the statuses, the status changes and where it is used are
/// rows of their own (WF-01, ADR-0031).
#[derive(Clone, Copy, Iden)]
pub(crate) enum WorkflowRow {
    #[iden = "workflow"]
    Table,
    Id,
    Name,
    Version,
}

/// A status of a workflow.
#[derive(Clone, Copy, Iden)]
pub(crate) enum WorkflowStatusRow {
    #[iden = "workflow_status"]
    Table,
    Workflow,
    Key,
    Name,
    StatusGroup,
    Position,
}

/// A status change of a workflow, with its rules as JSON text.
#[derive(Clone, Copy, Iden)]
pub(crate) enum WorkflowChangeRow {
    #[iden = "workflow_change"]
    Table,
    Workflow,
    FromStatus,
    ToStatus,
    Button,
    Rules,
    Position,
}

/// Where a workflow is used: a project (none for the whole organization) and an issue type (none
/// for all types).
#[derive(Clone, Copy, Iden)]
pub(crate) enum WorkflowUseRow {
    #[iden = "workflow_use"]
    Table,
    Project,
    IssueType,
    Workflow,
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
    /// The project the group was made in; none for the whole organization.
    Project,
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
    #[iden = "change_run"]
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

/// Where an issue type or a group of labels is offered (ISS-02, TAG-02): one row per place and
/// subject. `kind` and `subject` together name the subject, so one table serves both.
#[derive(Clone, Copy, Iden)]
pub(crate) enum OfferRow {
    #[iden = "offer"]
    Table,
    Kind,
    Subject,
    Project,
    Offered,
}

stored_tables! {
    ProjectRow => bb_core::project::Project {
        Id => id,
        Parent => parent,
        ShortName => short_name,
        Name => name,
        Version => version,
    },
    ProjectAliasRow => bb_core::project::ProjectAlias {
        ShortName => short_name,
        Project => project,
        RetiredAt => retired_at,
    },
    OfferRow => bb_core::offer::Offer {
        Kind => subject,
        Subject => subject,
        Project => project,
        Offered => offered,
    },
    IssueTypeRow => bb_core::issue::IssueType {
        Id => id,
        Name => name,
        Icon => icon,
        Project => project,
        Position => position,
        RetiredAt => retired_at,
        Version => version,
    },
    IssueRow => bb_core::issue::Issue {
        Id => id,
        Project => project,
        IssueType => issue_type,
        Code => code,
        Title => title,
        Description => description,
        Status => status,
        Priority => priority,
        EstimatedTime => estimated_time,
        TimeLeft => time_left,
        ReportedBy => reported_by,
        CreatedAt => created_at,
        UpdatedAt => updated_at,
        ClosedAt => closed_at,
        DeletedAt => deleted_at,
        Version => version,
    },
    AssignedPersonRow => bb_core::issue::AssignedPerson {
        Issue => issue,
        Person => person,
        AddedAt => added_at,
    },
    IssueAliasRow => bb_core::issue::IssueAlias {
        Project => project,
        Code => code,
        Issue => issue,
        RetiredAt => retired_at,
    },
    IssueCounterRow => bb_core::issue::IssueCounter {
        Project => project,
        Last => last,
    },
    RelationKindRow => bb_core::relation::RelationKind {
        Id => id,
        Forward => forward,
        Backward => backward,
        Position => position,
        RetiredAt => retired_at,
        Version => version,
    },
    FieldRow => bb_core::own_fields::OwnField {
        Id => id,
        Name => name,
        Help => help,
        Kind => kind,
        AppliesTo => applies_to,
        Rules => rules,
        DefaultValue => default_value,
        Look => look,
        NeverToAssistant => never_to_assistant,
        Position => position,
        RetiredAt => retired_at,
        Version => version,
    },
    LayoutRow => bb_core::layout::Layout {
        Id => id,
        Project => project,
        IssueType => issue_type,
        Kind => kind,
        Version => version,
    },
    LayoutSectionRow => bb_core::layout::LayoutSection {
        Layout => layout,
        Position => position,
        Tab => tab,
        Title => title,
    },
    LayoutFieldRow => bb_core::layout::LayoutField {
        Layout => layout,
        Field => field,
        Section => section,
        Position => position,
    },
    LayoutOffFieldRow => bb_core::layout::LayoutOffField {
        Layout => layout,
        Field => field,
    },
    LayoutFieldRoleRow => bb_core::layout::LayoutFieldRole {
        Layout => layout,
        Field => field,
        Role => role,
    },
    WorkflowRow => bb_core::workflow::Workflow {
        Id => id,
        Name => name,
        Version => version,
    },
    WorkflowStatusRow => bb_core::workflow::StoredStatus {
        Workflow => workflow,
        Key => key,
        Name => name,
        StatusGroup => group,
        Position => position,
    },
    WorkflowChangeRow => bb_core::workflow::StoredChange {
        Workflow => workflow,
        FromStatus => from,
        ToStatus => to,
        Button => button,
        Rules => rules,
        Position => position,
    },
    WorkflowUseRow => bb_core::workflow::WorkflowUse {
        Project => project,
        IssueType => issue_type,
        Workflow => workflow,
    },
    RelationRow => bb_core::relation::Relation {
        Id => id,
        Kind => kind,
        FromIssue => from,
        ToIssue => to,
        AddedBy => added_by,
        AddedAt => added_at,
    },
    IssueParentRow => bb_core::relation::IssueParent {
        Child => child,
        Parent => parent,
        SetBy => set_by,
        SetAt => set_at,
    },
    TagRow => bb_core::tags::Tag {
        Id => id,
        Name => name,
        NameKey => name,
        Color => color,
        CreatedAt => created_at,
        Version => version,
    },
    TagProjectRow => bb_core::tags::TagProject {
        Tag => tag,
        Project => project,
    },
    IssueTagRow => bb_core::tags::IssueTag {
        Issue => issue,
        Tag => tag,
        AddedBy => added_by,
        AddedAt => added_at,
    },
    LabelGroupRow => bb_core::tags::LabelGroup {
        Id => id,
        Name => name,
        NameKey => name,
        PickOne => pick_one,
        Project => project,
        Position => position,
        RetiredAt => retired_at,
        Version => version,
    },
    LabelRow => bb_core::tags::Label {
        Id => id,
        Group => group,
        GroupKey => group,
        Name => name,
        NameKey => name,
        Color => color,
        Position => position,
        RetiredAt => retired_at,
        Version => version,
    },
    IssueLabelRow => bb_core::tags::IssueLabel {
        Issue => issue,
        Label => label,
        AddedBy => added_by,
        AddedAt => added_at,
    },
    ManyChangeRunRow => bb_core::many_changes::ManyChangeRun {
        Id => id,
        Person => person,
        Action => action,
        Items => items,
        Status => status,
        Done => done,
        Changed => changed,
        Skipped => skipped,
        CreatedAt => created_at,
        FinishedAt => finished_at,
    },
    FieldProjectRow => bb_core::own_fields::FieldProject {
        Field => field,
        Project => project,
    },
    FieldIssueTypeRow => bb_core::own_fields::FieldIssueType {
        Field => field,
        IssueType => issue_type,
    },
    FieldChoiceRow => bb_core::own_fields::FieldChoice {
        Id => id,
        Field => field,
        Name => name,
        Number => number,
        Color => color,
        Icon => icon,
        Position => position,
        RetiredAt => retired_at,
    },
    FieldValueRow => bb_core::own_fields_value::StoredFieldValue {
        EntityKind => entity_kind,
        EntityId => entity_id,
        Field => field,
        ValueText => value_text,
        ValueNumber => value_number,
        ValueDate => value_date,
        ValueRef => value_ref,
        Position => position,
    },
    RoleTypeFieldLevelRow => bb_core::access::RoleTypeFieldLevel {
        Role => role,
        IssueType => issue_type,
        Field => field,
        Level => level,
    },
    ProjectTypeFieldLevelRow => bb_core::access::ProjectTypeFieldLevel {
        Project => project,
        Role => role,
        IssueType => issue_type,
        Field => field,
        Level => level,
    },
}
