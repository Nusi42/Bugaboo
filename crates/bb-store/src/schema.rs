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

mod access;
mod comments;
mod files;
mod notify;
mod sign_in;
mod system;
mod text_index;
mod work;

pub(crate) use access::*;
pub(crate) use comments::*;
pub(crate) use files::*;
pub(crate) use notify::*;
pub(crate) use sign_in::*;
pub(crate) use system::*;
pub(crate) use text_index::*;
pub(crate) use work::*;

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
    },
    StoredFileRow => bb_core::files::StoredFile {
        Hash => hash,
        Size => size,
        MediaType => media_type,
        Preview => preview,
        CreatedAt => created_at,
    },
    AttachmentRow => bb_core::files::Attachment {
        Id => id,
        Issue => issue,
        Comment => comment,
        File => file,
        Name => name,
        AddedBy => added_by,
        AddedAt => added_at,
        DeletedAt => deleted_at,
        Preview => preview,
        Version => version,
    },
    UploadRow => bb_core::files::Upload {
        Id => id,
        Issue => issue,
        Comment => comment,
        Person => person,
        Name => name,
        Size => size,
        CreatedAt => created_at,
        UpdatedAt => updated_at,
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
    TextIndexRow => bb_core::text_index::IndexedText {
        Id => id,
        Issue => issue,
        Source => source,
        SourceRef => source_ref,
        Words => words,
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
    NotificationRow => bb_core::notification::Notification {
        Id => id,
        Recipient => recipient,
        Kind => kind,
        Issue => issue,
        Event => event,
        Once => once,
        Record => record,
        By => by,
        At => at,
        Changes => changes,
        InApp => in_app,
        Mail => mail,
        ReadAt => read_at,
    },
    PersonHandleRow => bb_core::handle::PersonHandle {
        Person => person,
        Handle => handle,
        ChangedAt => changed_at,
        Current => current,
    },
    SignUpMailRow => bb_core::sign_up_invitation::SignUpMail {
        Address => address,
        Invitation => invitation,
        SentAt => sent_at,
    },
    SignUpInvitationRow => bb_core::sign_up_invitation::SignUpInvitation {
        Id => id,
        TokenHash => token_hash,
        Email => email,
        CreatedAt => created_at,
        ExpiresAt => expires_at,
        UsedAt => used_at,
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
