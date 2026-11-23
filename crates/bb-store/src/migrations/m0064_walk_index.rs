//! Migration 64: the index of the walk of a person's data (ADR-0044, decision 1, amendment *the
//! index of the walk*, M9-F144).
//!
//! For each table whose changes or JSON can name a person, a table of the rows whose text holds the
//! ID of a person: the key of the row and the person, the key first, so a row deleted takes its
//! rows of the index along (`ON DELETE CASCADE`) by one look, and a second index by the person,
//! which the walk reads. The tables are new, so no table that holds data is written again or
//! locked for long. The last step fills the index from the rows that are there, a page at a time.
//!
//! Beside it, every column through which a table names a person gets an index of its own where it
//! had none (the actor of an event, the author of a comment, who added a tag, …): the walk asks
//! each of them in an arm of its own (`person_data/arms.rs`), and one arm without an index reads
//! its whole table. The notifications get an index by their event, which the change of a removal
//! looks them up by (M9-F51).
//!
//! Last, the table of the removals that are not finished (`removal_pending`, ADR-0044, decision 6):
//! the job that finishes a removal goes by its row, and a person is being removed while it exists.
//! It is new and empty: a removal of an earlier version was finished by its change.

use sea_query::{
    ColumnDef, ForeignKey, ForeignKeyAction, Iden, Index, Table, TableCreateStatement,
};

use super::{Fill, Step, id_column};
use crate::schema::{
    AcceptedWarningRow, AgentRow, AiAgreementRow, AssignedPersonRow, AttachmentRow, AwayTimeRow,
    BackupRunRow, ChangeRunNamesPerson, CommentRow, DataExportRow, EditLockRow, EditNoticeRow,
    Event, EventNamesPerson, HandOverRow, HistoryEntryRow, HistoryNamesPerson, Invitation,
    IssueLabelRow, IssueParentRow, IssueRow, IssueTagRow, IssueWorkRow, Job, JobNamesPerson,
    ManyChangeRunRow, NotificationNamesPerson, NotificationRow, OAuthWebAddressRow, OwnThemeRow,
    PersonRow, PersonStandingRow, ProposalNamesPerson, ProposalRow, QuestionRow, RelationRow,
    RemovalPendingRow, ReviewRow, SecurityLog, StandingRuleRow, TimeFactorRow, ToolCallRow,
};

/// The two steps of the index of a table keyed by one ID: the table and its index by person.
fn index_of(
    table: &mut TableCreateStatement,
    index: impl Iden + Copy + 'static,
    of: impl Iden + Copy + 'static,
    key: impl Iden + Copy + 'static,
) -> [Step; 2] {
    let name = index.to_string();
    [
        Step::Table(
            table
                .col(id_column(EventNamesPerson::Row))
                .col(id_column(EventNamesPerson::Person))
                .primary_key(
                    Index::create()
                        .col(EventNamesPerson::Row)
                        .col(EventNamesPerson::Person),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(index, EventNamesPerson::Row)
                        .to(of, key)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name(format!("{name}_by_person"))
                .table(index)
                .col(EventNamesPerson::Person)
                .col(EventNamesPerson::Row)
                .to_owned(),
        ),
    ]
}

/// An index of a table by the columns through which it names a person.
macro_rules! by_person {
    ($name:literal, $table:expr, $($column:expr),+ $(,)?) => {
        Step::Index(
            Index::create()
                .name($name)
                .table($table)
                $(.col($column))+
                .to_owned(),
        )
    };
}

/// The indexes of the columns that name a person and had none: those of the work on issues,
/// then [`other_person_columns`].
fn person_columns() -> Vec<Step> {
    let mut steps = vec![
        by_person!("event_by_actor", Event::Table, Event::ActorId),
        by_person!(
            "history_entry_by_actor",
            HistoryEntryRow::Table,
            HistoryEntryRow::By
        ),
        by_person!("comment_by_author", CommentRow::Table, CommentRow::Author),
        by_person!(
            "notification_by_actor",
            NotificationRow::Table,
            NotificationRow::By
        ),
        by_person!(
            "issue_by_reporter_alone",
            IssueRow::Table,
            IssueRow::ReportedBy
        ),
        by_person!(
            "assigned_person_by_person",
            AssignedPersonRow::Table,
            AssignedPersonRow::Person
        ),
        by_person!(
            "attachment_by_adder",
            AttachmentRow::Table,
            AttachmentRow::AddedBy
        ),
        by_person!(
            "relation_by_adder",
            RelationRow::Table,
            RelationRow::AddedBy
        ),
        by_person!(
            "issue_parent_by_setter",
            IssueParentRow::Table,
            IssueParentRow::SetBy
        ),
        by_person!(
            "issue_tag_by_adder",
            IssueTagRow::Table,
            IssueTagRow::AddedBy
        ),
        by_person!(
            "issue_label_by_adder",
            IssueLabelRow::Table,
            IssueLabelRow::AddedBy
        ),
        by_person!(
            "security_log_by_person",
            SecurityLog::Table,
            SecurityLog::Person
        ),
        by_person!(
            "security_log_by_subject",
            SecurityLog::Table,
            SecurityLog::SubjectKind,
            SecurityLog::SubjectId,
        ),
    ];
    steps.extend(other_person_columns());
    steps
}

/// The indexes of the columns of agents that name a person and had none, then
/// [`sign_in_and_setting_columns`].
fn other_person_columns() -> Vec<Step> {
    let mut steps = vec![
        by_person!(
            "issue_work_by_person",
            IssueWorkRow::Table,
            IssueWorkRow::Person
        ),
        by_person!(
            "tool_call_by_person",
            ToolCallRow::Table,
            ToolCallRow::Person
        ),
        by_person!(
            "change_run_by_person",
            ManyChangeRunRow::Table,
            ManyChangeRunRow::Person
        ),
        by_person!(
            "agent_question_by_answerer",
            QuestionRow::Table,
            QuestionRow::AnsweredBy
        ),
        by_person!("agent_review_by_reviewer", ReviewRow::Table, ReviewRow::By),
        by_person!(
            "agent_proposal_by_decider",
            ProposalRow::Table,
            ProposalRow::DecidedBy
        ),
        by_person!(
            "hand_over_by_person",
            HandOverRow::Table,
            HandOverRow::Person
        ),
        by_person!(
            "standing_rule_by_person",
            StandingRuleRow::Table,
            StandingRuleRow::Person
        ),
        by_person!("agent_by_offered_to", AgentRow::Table, AgentRow::OfferedTo),
        by_person!("agent_by_maker", AgentRow::Table, AgentRow::MadeBy),
    ];
    steps.extend(sign_in_and_setting_columns());
    steps
}

/// The indexes of the columns of sign-in, settings and records of the organization that name a
/// person and had none.
fn sign_in_and_setting_columns() -> Vec<Step> {
    vec![
        by_person!(
            "backup_run_by_person",
            BackupRunRow::Table,
            BackupRunRow::Person
        ),
        by_person!(
            "data_export_by_asker",
            DataExportRow::Table,
            DataExportRow::AskedBy
        ),
        by_person!(
            "oauth_web_address_by_adder",
            OAuthWebAddressRow::Table,
            OAuthWebAddressRow::AddedBy
        ),
        by_person!(
            "issue_time_factor_by_person",
            TimeFactorRow::Table,
            TimeFactorRow::Person
        ),
        by_person!(
            "issue_time_factor_by_setter",
            TimeFactorRow::Table,
            TimeFactorRow::SetBy
        ),
        by_person!(
            "accepted_warning_by_person",
            AcceptedWarningRow::Table,
            AcceptedWarningRow::AcceptedBy
        ),
        by_person!(
            "person_standing_by_actor",
            PersonStandingRow::Table,
            PersonStandingRow::By
        ),
        by_person!(
            "invitation_by_address",
            Invitation::Table,
            Invitation::Email
        ),
        by_person!(
            "invitation_by_inviter",
            Invitation::Table,
            Invitation::InvitedBy
        ),
        by_person!(
            "edit_lock_by_holder",
            EditLockRow::Table,
            EditLockRow::Holder
        ),
        by_person!(
            "edit_notice_by_sender",
            EditNoticeRow::Table,
            EditNoticeRow::FromPerson
        ),
        by_person!(
            "ai_agreement_by_person",
            AiAgreementRow::Table,
            AiAgreementRow::ConfirmedBy
        ),
        by_person!(
            "own_theme_by_maker_alone",
            OwnThemeRow::Table,
            OwnThemeRow::Maker
        ),
        by_person!(
            "away_time_by_enterer",
            AwayTimeRow::Table,
            AwayTimeRow::EnteredBy
        ),
    ]
}

pub(super) fn steps() -> Vec<Step> {
    let mut steps = person_columns();
    // The notifications of the events about a person's records, which a removal scrubs in its
    // change, and of the events of the comments it deletes, are found by their event.
    steps.push(by_person!(
        "notification_by_event",
        NotificationRow::Table,
        NotificationRow::Event
    ));
    // The comments in the trash, which no search finds: the comments of a removal wait there
    // until its job deletes them.
    steps.push(by_person!(
        "comment_in_trash",
        CommentRow::Table,
        CommentRow::DeletedAt
    ));
    steps.extend(index_of(
        Table::create().table(EventNamesPerson::Table),
        EventNamesPerson::Table,
        Event::Table,
        Event::Id,
    ));
    steps.extend(index_of(
        Table::create().table(NotificationNamesPerson::Table),
        NotificationNamesPerson::Table,
        NotificationRow::Table,
        NotificationRow::Id,
    ));
    steps.extend(index_of(
        Table::create().table(JobNamesPerson::Table),
        JobNamesPerson::Table,
        Job::Table,
        Job::Id,
    ));
    steps.extend(index_of(
        Table::create().table(ChangeRunNamesPerson::Table),
        ChangeRunNamesPerson::Table,
        ManyChangeRunRow::Table,
        ManyChangeRunRow::Id,
    ));
    steps.extend(index_of(
        Table::create().table(ProposalNamesPerson::Table),
        ProposalNamesPerson::Table,
        ProposalRow::Table,
        ProposalRow::Id,
    ));
    steps.push(Step::Table(
        Table::create()
            .table(HistoryNamesPerson::Table)
            .col(id_column(HistoryNamesPerson::Event))
            .col(id_column(HistoryNamesPerson::Issue))
            .col(id_column(HistoryNamesPerson::Person))
            .primary_key(
                Index::create()
                    .col(HistoryNamesPerson::Event)
                    .col(HistoryNamesPerson::Issue)
                    .col(HistoryNamesPerson::Person),
            )
            .foreign_key(
                ForeignKey::create()
                    .from_tbl(HistoryNamesPerson::Table)
                    .from_col(HistoryNamesPerson::Event)
                    .from_col(HistoryNamesPerson::Issue)
                    .to_tbl(HistoryEntryRow::Table)
                    .to_col(HistoryEntryRow::Event)
                    .to_col(HistoryEntryRow::Issue)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .to_owned(),
    ));
    steps.push(Step::Index(
        Index::create()
            .name("history_entry_names_person_by_person")
            .table(HistoryNamesPerson::Table)
            .col(HistoryNamesPerson::Person)
            .col(HistoryNamesPerson::Event)
            .col(HistoryNamesPerson::Issue)
            .to_owned(),
    ));
    steps.push(Step::Fill(Fill::RowNamingPersons));
    steps.push(pending_removals());
    steps
}

/// The table of the removals that are not finished: one row for each, from the change of the
/// removal to the last step of its job.
fn pending_removals() -> Step {
    Step::Table(
        Table::create()
            .table(RemovalPendingRow::Table)
            .col(id_column(RemovalPendingRow::Person).primary_key())
            .col(
                ColumnDef::new(RemovalPendingRow::CommentsGo)
                    .boolean()
                    .not_null(),
            )
            .col(id_column(RemovalPendingRow::Job))
            .col(ColumnDef::new(RemovalPendingRow::Step).text().not_null())
            .col(
                ColumnDef::new(RemovalPendingRow::CreatedAt)
                    .big_integer()
                    .not_null(),
            )
            .col(
                ColumnDef::new(RemovalPendingRow::UpdatedAt)
                    .big_integer()
                    .not_null(),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(RemovalPendingRow::Table, RemovalPendingRow::Person)
                    .to(PersonRow::Table, PersonRow::Id),
            )
            .to_owned(),
    )
}
