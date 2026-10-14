//! The names of tables and columns, written once. Migrations create the tables from them and
//! repositories query them; no table or column name is written as text anywhere else.
//!
//! Every table belongs to an entity with personal-data marks (ADR-0015): [`stored_tables!`] ties
//! them together, column by column (`Column => field`), so a table without an entity does not
//! compile, and DATA-MARKS reads the same list to find a table that is created without being
//! listed and a column that no marked field stands behind.
//!
//! All whole numbers are `BIGINT` and all moments are microseconds since 1970 in a `BIGINT`, so
//! both databases hand back the same Rust types. IDs are 16-byte blobs. JSON is stored as text.

use sea_query::{Iden, IdenStatic};

#[derive(Clone, Copy, Iden)]
pub(crate) enum Event {
    Table,
    Id,
    Time,
    ActorKind,
    ActorId,
    Channel,
    EntityKind,
    EntityId,
    Changes,
    Reason,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum Outbox {
    Table,
    Position,
    EventId,
    CreatedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum OutboxPosition {
    Table,
    Consumer,
    Position,
    UpdatedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum Job {
    Table,
    Id,
    Kind,
    Payload,
    RunAt,
    Attempts,
    MaxAttempts,
    State,
    LeaseOwner,
    LeaseUntil,
    LastFailure,
    CreatedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum SecurityLog {
    Table,
    Seq,
    Time,
    Person,
    Action,
    SubjectKind,
    SubjectId,
    Network,
    PreviousHash,
    Hash,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum SecurityLogCheckpoint {
    Table,
    Seq,
    Hash,
    Signature,
    CreatedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum Secret {
    Table,
    Name,
    KeyId,
    WrappedKey,
    WrappedNonce,
    Nonce,
    Ciphertext,
    UpdatedAt,
}

/// The lease on one thing being edited. A row lives only while its lease runs (ADR-0019).
#[derive(Clone, Copy, Iden)]
pub(crate) enum EditLockRow {
    #[iden = "edit_lock"]
    Table,
    RecordKind,
    RecordId,
    Part,
    Holder,
    Editor,
    Token,
    TakenAt,
    ExpiresAt,
}

/// The unsaved text of a person (a draft), kept until it is saved, thrown away, or 30 days old.
#[derive(Clone, Copy, Iden)]
pub(crate) enum EditDraftRow {
    #[iden = "edit_draft"]
    Table,
    Author,
    Target,
    Text,
    BaseVersion,
    SavedAt,
}

/// A note for the person who was editing, read at their next heartbeat.
#[derive(Clone, Copy, Iden)]
pub(crate) enum EditNoticeRow {
    #[iden = "edit_notice"]
    Table,
    Recipient,
    RecordKind,
    RecordId,
    Part,
    Kind,
    FromPerson,
    At,
}

/// The values of settings. Only `settings.rs` reads or writes this table (ARCH-SETTINGS).
#[derive(Clone, Copy, Iden)]
pub(crate) enum SettingRow {
    #[iden = "setting"]
    Table,
    Key,
    Level,
    Scope,
    Value,
    Locked,
    UpdatedAt,
}

/// The version of a policy that is made of several settings: a row of its own, so that a change or
/// a reset of a setting cannot take the version back.
#[derive(Clone, Copy, Iden)]
pub(crate) enum PolicyVersionRow {
    #[iden = "policy_version"]
    Table,
    Policy,
    Version,
}

/// The values of settings that people chose for themselves in *My settings*. Only `settings.rs`
/// reads or writes this table (ARCH-SETTINGS); it is its own table so the person it belongs to is
/// a column of its own with a personal-data mark.
#[derive(Clone, Copy, Iden)]
pub(crate) enum PersonSettingRow {
    #[iden = "person_setting"]
    Table,
    Person,
    Key,
    Value,
    UpdatedAt,
}

/// The people who can sign in. The password is not here: it has its own table, so no query for a
/// person can hand a hash to code that does not need it.
#[derive(Clone, Copy, Iden)]
pub(crate) enum PersonRow {
    #[iden = "person"]
    Table,
    Id,
    DisplayName,
    Email,
    CreatedAt,
}

/// Columns that an earlier migration made and a later one took away. Old migrations keep their
/// text (a changed migration is refused), so the name stays here; no entity stands behind it.
#[derive(Clone, Copy, Iden)]
pub(crate) enum Retired {
    Administrator,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum PasswordLogin {
    Table,
    Person,
    Hash,
    ChangedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum SignInSession {
    Table,
    Id,
    Person,
    TokenHash,
    Stage,
    CreatedAt,
    LastSeenAt,
    Network,
    Device,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum TwoStep {
    Table,
    Person,
    Confirmed,
    LastStep,
    CreatedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum BackupCode {
    Table,
    Person,
    CodeHash,
    UsedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum PasskeyRow {
    #[iden = "passkey"]
    Table,
    Id,
    Person,
    CredentialId,
    Data,
    Label,
    CreatedAt,
    LastUsedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum Invitation {
    Table,
    Id,
    TokenHash,
    Email,
    InvitedBy,
    CreatedAt,
    ExpiresAt,
    UsedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum SetupCodeRow {
    #[iden = "setup_code"]
    Table,
    CodeHash,
    CreatedAt,
}

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
pub(crate) enum IssueAssigneeRow {
    #[iden = "issue_assignee"]
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
pub(crate) enum BulkRunRow {
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

/// Comments under issues (ISS-05).
#[derive(Clone, Copy, Iden)]
pub(crate) enum CommentRow {
    #[iden = "comment"]
    Table,
    Id,
    Issue,
    Author,
    Text,
    CreatedAt,
    EditedAt,
    DeletedAt,
    Version,
}

/// The earlier texts of edited comments (ISS-05).
#[derive(Clone, Copy, Iden)]
pub(crate) enum CommentVersionRow {
    #[iden = "comment_version"]
    Table,
    Comment,
    Version,
    Text,
    SavedAt,
}

/// The people a comment puts into *Mentions* (ISS-05).
#[derive(Clone, Copy, Iden)]
pub(crate) enum MentionRow {
    #[iden = "mention"]
    Table,
    Comment,
    Recipient,
    Via,
    ReadAt,
}

/// The people who follow an issue (ISS-07).
#[derive(Clone, Copy, Iden)]
pub(crate) enum FollowerRow {
    #[iden = "follower"]
    Table,
    Issue,
    Person,
    Reason,
    Since,
}

/// The history of issues, built from the events by an outbox consumer (ISS-06).
#[derive(Clone, Copy, Iden)]
pub(crate) enum HistoryEntryRow {
    #[iden = "history_entry"]
    Table,
    Event,
    Issue,
    Position,
    At,
    By,
    Kind,
    Record,
    RecordKind,
    Changes,
}

/// Groups of people. The table is called `person_group`: `group` is a word of SQL.
#[derive(Clone, Copy, Iden)]
pub(crate) enum GroupRow {
    #[iden = "person_group"]
    Table,
    Id,
    Name,
    Version,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum GroupMemberRow {
    #[iden = "group_member"]
    Table,
    Group,
    Person,
    AddedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum RoleRow {
    #[iden = "role"]
    Table,
    Id,
    Name,
    Starting,
    Position,
    TwoStep,
    Version,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum RolePermissionRow {
    #[iden = "role_permission"]
    Table,
    Role,
    Permission,
}

/// *Who has which role*: a person or a group, a role or *No access* (no role), and where it counts.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AssignmentRow {
    #[iden = "assignment"]
    Table,
    Id,
    Person,
    Group,
    Role,
    Project,
    AppliesTo,
    GivenAt,
}

#[derive(Clone, Copy, IdenStatic)]
pub(crate) enum RoleFieldLevelRow {
    #[iden = "role_field_level"]
    Table,
    Role,
    Field,
    Level,
}

#[derive(Clone, Copy, IdenStatic)]
pub(crate) enum ProjectFieldLevelRow {
    #[iden = "project_field_level"]
    Table,
    Project,
    Role,
    Field,
    Level,
}

#[derive(Clone, Copy, IdenStatic)]
pub(crate) enum RolePeopleLevelRow {
    #[iden = "role_people_level"]
    Table,
    Role,
    Kind,
    Level,
}

#[derive(Clone, Copy, IdenStatic)]
pub(crate) enum ProjectPeopleLevelRow {
    #[iden = "project_people_level"]
    Table,
    Project,
    Role,
    Kind,
    Level,
}

/// The access keys of people (AUTH-07). Only the hash of the secret is stored.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AccessKeyRow {
    #[iden = "access_key"]
    Table,
    Id,
    Owner,
    Name,
    Prefix,
    SecretHash,
    Choice,
    CreatedAt,
    ExpiresAt,
    LastUsedAt,
}

/// The single permissions of a key whose choice is *More options*.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AccessKeyPermissionRow {
    #[iden = "access_key_permission"]
    Table,
    Key,
    Permission,
}

/// The projects a key is limited to.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AccessKeyProjectRow {
    #[iden = "access_key_project"]
    Table,
    Key,
    Project,
}

/// The people who were administrators in M1, until the first start gives them the role.
#[derive(Clone, Copy, Iden)]
pub(crate) enum LegacyAdministratorRow {
    #[iden = "legacy_administrator"]
    Table,
    Person,
}

/// One row, once the starting roles were made.
#[derive(Clone, Copy, Iden)]
pub(crate) enum StartingRolesMadeRow {
    #[iden = "starting_roles_made"]
    Table,
    MadeAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum Migration {
    Table,
    Version,
    Name,
    Checksum,
    AppliedAt,
}

/// Names given to computed columns in a query, so they are read back like any other column.
#[derive(Clone, Copy, Iden)]
pub(crate) enum Computed {
    Total,
    Block,
    First,
    Last,
    Newest,
}

/// A table with the entity stored in it.
#[derive(Debug)]
pub struct StoredTable {
    /// The name of the table.
    pub name: String,
    /// The personal-data marks of the entity stored in it.
    pub marks: bb_core::personal::EntityMarks,
    /// Each column of the table with the field of the entity it holds (several columns can hold
    /// one field, for example a kind and an ID).
    pub columns: Vec<(String, &'static str)>,
}

/// Names the entity of every table and the field behind every column:
/// `stored_tables! { Event => DomainEvent { Id => id, … } }`. The name before the first arrow
/// is the table's enum in this file, and the names before the other arrows are its variants, so
/// a column that does not exist does not compile; DATA-MARKS requires that every variant has a
/// line and that the field exists in the entity.
macro_rules! stored_tables {
    ($($table:ident => $entity:ty {
        $($column:ident => $field:ident),+ $(,)?
    }),+ $(,)?) => {
        /// The name of every table with the marks of the entity stored in it and the field
        /// behind each column, for the personal-data map.
        #[must_use]
        pub fn tables() -> Vec<StoredTable> {
            vec![$(StoredTable {
                name: sea_query::Iden::to_string(&$table::Table),
                marks: <$entity as bb_core::personal::Entity>::MARKS,
                columns: vec![$((
                    sea_query::Iden::to_string(&$table::$column),
                    stringify!($field),
                )),+],
            }),+]
        }
    };
}

stored_tables! {
    Event => bb_core::event::DomainEvent {
        Id => id,
        Time => time,
        ActorKind => actor,
        ActorId => actor,
        Channel => channel,
        EntityKind => entity,
        EntityId => entity,
        Changes => changes,
        Reason => reason,
    },
    Job => bb_core::job::Job {
        Id => id,
        Kind => kind,
        Payload => payload,
        RunAt => run_at,
        Attempts => attempts,
        MaxAttempts => max_attempts,
        State => state,
        LeaseOwner => lease_owner,
        LeaseUntil => lease_until,
        LastFailure => last_failure,
        CreatedAt => created_at,
    },
    Migration => bb_core::migration::MigrationRecord {
        Version => version,
        Name => name,
        Checksum => checksum,
        AppliedAt => applied_at,
    },
    Outbox => bb_core::outbox::OutboxEntry {
        Position => position,
        EventId => event_id,
        CreatedAt => created_at,
    },
    OutboxPosition => bb_core::outbox::OutboxPosition {
        Consumer => consumer,
        Position => position,
        UpdatedAt => updated_at,
    },
    Secret => bb_core::secret::StoredSecret {
        Name => name,
        KeyId => key_id,
        WrappedKey => wrapped_key,
        WrappedNonce => wrapped_nonce,
        Nonce => nonce,
        Ciphertext => ciphertext,
        UpdatedAt => updated_at,
    },
    SecurityLog => bb_core::security_log::SecurityLogEntry {
        Seq => seq,
        Time => time,
        Person => person,
        Action => action,
        SubjectKind => subject,
        SubjectId => subject,
        Network => network,
        PreviousHash => previous_hash,
        Hash => hash,
    },
    SecurityLogCheckpoint => bb_core::security_log::SecurityLogCheckpoint {
        Seq => seq,
        Hash => hash,
        Signature => signature,
        CreatedAt => created_at,
    },
    PersonRow => bb_core::person::Person {
        Id => id,
        DisplayName => display_name,
        Email => email,
        CreatedAt => created_at,
    },
    PasswordLogin => bb_core::sign_in::PasswordLogin {
        Person => person,
        Hash => hash,
        ChangedAt => changed_at,
    },
    SignInSession => bb_core::sign_in::SignInSession {
        Id => id,
        Person => person,
        TokenHash => token_hash,
        Stage => stage,
        CreatedAt => created_at,
        LastSeenAt => last_seen_at,
        Network => network,
        Device => device,
    },
    TwoStep => bb_core::sign_in::TwoStep {
        Person => person,
        Confirmed => confirmed,
        LastStep => last_step,
        CreatedAt => created_at,
    },
    BackupCode => bb_core::sign_in::BackupCode {
        Person => person,
        CodeHash => code_hash,
        UsedAt => used_at,
    },
    PasskeyRow => bb_core::sign_in::StoredPasskey {
        Id => id,
        Person => person,
        CredentialId => credential_id,
        Data => data,
        Label => label,
        CreatedAt => created_at,
        LastUsedAt => last_used_at,
    },
    Invitation => bb_core::sign_in::Invitation {
        Id => id,
        TokenHash => token_hash,
        Email => email,
        InvitedBy => invited_by,
        CreatedAt => created_at,
        ExpiresAt => expires_at,
        UsedAt => used_at,
    },
    SetupCodeRow => bb_core::sign_in::SetupCode {
        CodeHash => code_hash,
        CreatedAt => created_at,
    },
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
    IssueTypeRow => bb_core::issue::IssueType {
        Id => id,
        Name => name,
        Icon => icon,
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
        ReportedBy => reported_by,
        CreatedAt => created_at,
        UpdatedAt => updated_at,
        ClosedAt => closed_at,
        DeletedAt => deleted_at,
        Version => version,
    },
    IssueAssigneeRow => bb_core::issue::IssueAssignee {
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
    LayoutFieldRoleRow => bb_core::layout::LayoutFieldRole {
        Layout => layout,
        Field => field,
        Role => role,
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
    BulkRunRow => bb_core::bulk::BulkRun {
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
    CommentRow => bb_core::comment::Comment {
        Id => id,
        Issue => issue,
        Author => author,
        Text => text,
        CreatedAt => created_at,
        EditedAt => edited_at,
        DeletedAt => deleted_at,
        Version => version,
    },
    CommentVersionRow => bb_core::comment::CommentVersion {
        Comment => comment,
        Version => version,
        Text => text,
        SavedAt => saved_at,
    },
    MentionRow => bb_core::comment::Mention {
        Comment => comment,
        Recipient => recipient,
        Via => via,
        ReadAt => read_at,
    },
    FollowerRow => bb_core::follow::Follower {
        Issue => issue,
        Person => person,
        Reason => reason,
        Since => since,
    },
    HistoryEntryRow => bb_core::history::HistoryEntry {
        Event => event,
        Issue => issue,
        Position => position,
        At => at,
        By => by,
        Kind => kind,
        Record => record,
        RecordKind => record_kind,
        Changes => changes,
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
    GroupRow => bb_core::access::Group {
        Id => id,
        Name => name,
        Version => version,
    },
    GroupMemberRow => bb_core::access::GroupMember {
        Group => group,
        Person => person,
        AddedAt => added_at,
    },
    RoleRow => bb_core::access::Role {
        Id => id,
        Name => name,
        Starting => starting,
        Position => position,
        TwoStep => two_step,
        Version => version,
    },
    RolePermissionRow => bb_core::access::RolePermission {
        Role => role,
        Permission => permission,
    },
    AssignmentRow => bb_core::access::Assignment {
        Id => id,
        Person => person,
        Group => group,
        Role => role,
        Project => project,
        AppliesTo => applies_to,
        GivenAt => given_at,
    },
    RoleFieldLevelRow => bb_core::access::RoleFieldLevel {
        Role => role,
        Field => field,
        Level => level,
    },
    ProjectFieldLevelRow => bb_core::access::ProjectFieldLevel {
        Project => project,
        Role => role,
        Field => field,
        Level => level,
    },
    RolePeopleLevelRow => bb_core::access::RolePeopleLevel {
        Role => role,
        Kind => kind,
        Level => level,
    },
    ProjectPeopleLevelRow => bb_core::access::ProjectPeopleLevel {
        Project => project,
        Role => role,
        Kind => kind,
        Level => level,
    },
    LegacyAdministratorRow => bb_core::access::LegacyAdministrator {
        Person => person,
    },
    StartingRolesMadeRow => bb_core::access::StartingRolesMade {
        MadeAt => made_at,
    },
    AccessKeyRow => bb_core::access_key::AccessKey {
        Id => id,
        Owner => owner,
        Name => name,
        Prefix => prefix,
        SecretHash => secret_hash,
        Choice => choice,
        CreatedAt => created_at,
        ExpiresAt => expires_at,
        LastUsedAt => last_used_at,
    },
    AccessKeyPermissionRow => bb_core::access_key::AccessKeyPermission {
        Key => key,
        Permission => permission,
    },
    AccessKeyProjectRow => bb_core::access_key::AccessKeyProject {
        Key => key,
        Project => project,
    },
    EditLockRow => bb_core::edit::EditLock {
        RecordKind => record,
        RecordId => record,
        Part => part,
        Holder => holder,
        Editor => editor,
        Token => token,
        TakenAt => taken_at,
        ExpiresAt => expires_at,
    },
    EditDraftRow => bb_core::edit::Draft {
        Author => author,
        Target => target,
        Text => text,
        BaseVersion => base_version,
        SavedAt => saved_at,
    },
    EditNoticeRow => bb_core::edit::EditNotice {
        Recipient => recipient,
        RecordKind => record,
        RecordId => record,
        Part => part,
        Kind => kind,
        FromPerson => from,
        At => at,
    },
    SettingRow => bb_core::settings::StoredSetting {
        Key => key,
        Level => level,
        Scope => scope,
        Value => value,
        Locked => locked,
        UpdatedAt => updated_at,
    },
    PolicyVersionRow => bb_core::settings::PolicyVersion {
        Policy => policy,
        Version => version,
    },
    PersonSettingRow => bb_core::settings::StoredPersonSetting {
        Person => person,
        Key => key,
        Value => value,
        UpdatedAt => updated_at,
    },
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_table_has_its_own_name_and_every_column_a_marked_field() {
        let tables = tables();
        let names: HashSet<&str> = tables.iter().map(|table| table.name.as_str()).collect();
        assert_eq!(names.len(), tables.len());
        assert!(names.contains("setting"));
        assert!(names.contains("security_log_checkpoint"));
        for table in &tables {
            assert!(!table.marks.fields().is_empty(), "{}", table.name);
            let fields: HashSet<&str> = table
                .marks
                .fields()
                .iter()
                .map(bb_core::personal::FieldMark::name)
                .collect();
            for (column, field) in &table.columns {
                assert!(
                    fields.contains(field),
                    "{}.{column} points to the field `{field}`, which the entity does not have",
                    table.name
                );
            }
            // Every field of the entity is stored in some column.
            let stored: HashSet<&str> = table.columns.iter().map(|(_, field)| *field).collect();
            for field in &fields {
                assert!(
                    stored.contains(field),
                    "{}: `{field}` is not stored",
                    table.name
                );
            }
        }
    }
}
