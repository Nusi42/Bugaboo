//! What *Remove a person* does with each kind of record that names the person (PRIV-04, ADR-0044,
//! decision 6): the one table the plan of a removal reads. A kind is deleted with the account when
//! it only serves the person, kept by law for its keep period (ADR-0045), or kept as a record of the
//! organization, which shows the person as *Former user* from then on. Every kind that is not named
//! here is kept: deleting a record of the organization has to be decided, never fallen into.

use jiff::Timestamp;

use crate::id::{JobId, UserId};
use crate::keeping::KeepKind;
use crate::personal::{Entity, EntityMarks, Link};

/// What a removal does with the records of one kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtRemoval {
    /// The rows in which the person is one of `by` are deleted in the change that removes the
    /// person: they only served their account. Rows that name the person in another way (a
    /// notification they caused for somebody else, an agent of another person they made) are
    /// kept, shown with *Former user*.
    Deleted {
        /// What the person is to a row that goes.
        by: &'static [Link],
    },
    /// Kept; the person is shown as *Former user* in it.
    KeptAsFormerUser,
    /// Kept for the keep period of this kind of the keep rules, then deleted by them.
    KeptByLaw(KeepKind),
}

const fn marks<E: Entity>() -> EntityMarks {
    E::MARKS
}

/// A kind of record of the account and what the person is to a row of it that goes.
const fn deleted<E: Entity>(by: &'static [Link]) -> (EntityMarks, &'static [Link]) {
    (E::MARKS, by)
}

/// The person holds it, is its subject, wrote it or is its recipient: their own row.
const OWN: &[Link] = &[Link::Holder, Link::Subject, Link::Author, Link::Recipient];
const HOLDER: &[Link] = &[Link::Holder];
const SUBJECT: &[Link] = &[Link::Subject];
const AUTHOR: &[Link] = &[Link::Author];
const RECIPIENT: &[Link] = &[Link::Recipient];

/// The kinds that only serve the person's account (ADR-0044, decision 6), each with what the
/// person is to a row that goes: sign-in, keys and grants, handle, settings, private saved
/// searches and their e-mailed searches, drafts, locks and the notes for and from them, the notifications they get and their
/// follows, work times and the away times that are theirs, the agents they hold and their standing
/// rules, the requests of programs for agents asked of them, the hand-overs they made, the rows
/// that give them roles (their memberships end), the
/// invitations they made or got (sign-in records of the account, used or not), and the files of
/// *Download my data* made of them, and the jobs whose payload names them. A note of an edit goes
/// where the person is its recipient; a note they caused for somebody else stays, like a
/// notification. Rows that name nobody themselves
/// (the projects and permissions of a key or a grant, the codes and tokens of a grant, the columns
/// of a search) go with the row they belong to, through the module that deletes it.
const DELETED: &[(EntityMarks, &[Link])] = &[
    deleted::<crate::sign_in::PasswordLogin>(HOLDER),
    deleted::<crate::sign_in::SignInSession>(HOLDER),
    deleted::<crate::sign_in::TwoStep>(HOLDER),
    deleted::<crate::sign_in::BackupCode>(HOLDER),
    deleted::<crate::sign_in::StoredPasskey>(HOLDER),
    deleted::<crate::access_key::AccessKey>(HOLDER),
    deleted::<crate::oauth::OAuthGrant>(HOLDER),
    deleted::<crate::handle::PersonHandle>(SUBJECT),
    deleted::<crate::settings::StoredPersonSetting>(SUBJECT),
    deleted::<crate::settings::PersonSettingCounter>(SUBJECT),
    deleted::<crate::saved_search::SavedSearch>(AUTHOR),
    deleted::<crate::dashboard::Dashboard>(AUTHOR),
    deleted::<crate::search_mail::SearchMail>(HOLDER),
    deleted::<crate::edit::Draft>(AUTHOR),
    deleted::<crate::edit::EditLock>(HOLDER),
    deleted::<crate::edit::EditNotice>(RECIPIENT),
    deleted::<crate::notification::Notification>(RECIPIENT),
    deleted::<crate::follow::Follower>(SUBJECT),
    deleted::<crate::pin::Pin>(HOLDER),
    deleted::<crate::work_times::record::UsualWorkDay>(SUBJECT),
    deleted::<crate::work_times::record::AwayTime>(SUBJECT),
    deleted::<crate::work_times::record::ProjectShare>(SUBJECT),
    deleted::<crate::time::entry::Timer>(SUBJECT),
    deleted::<crate::agent::Agent>(HOLDER),
    deleted::<crate::agent_request::AgentRequest>(HOLDER),
    deleted::<crate::agent_records::StandingRule>(AUTHOR),
    deleted::<crate::agent_records::HandOver>(&[Link::Actor]),
    deleted::<crate::access::Assignment>(HOLDER),
    deleted::<crate::access::GroupMember>(SUBJECT),
    deleted::<crate::sign_in::Invitation>(&[Link::Author, Link::Recipient]),
    deleted::<crate::data_export::DataExport>(OWN),
    // A job that waits names the person in its payload (a queued e-mail to them); one that names
    // them only by the address is found by it (`RemovalStore::delete_jobs_with_address`).
    deleted::<crate::job::Job>(SUBJECT),
];

/// A removal that is not finished yet (ADR-0044, decision 6): the change of the removal writes it
/// and the job `privacy.finish-removal` deletes it with its last step, so a person is being removed
/// exactly while this row exists. The name, the address and the handles the job still scrubs from
/// the changes are no part of it: they are a stored secret of their own (`privacy.removal.<ID>`),
/// encrypted with a key of their own, which goes with the row. While the row exists, the comments that go are already hidden (in the trash, kept from being
/// restored and from every search), but the person's name, handles and address may still stand in
/// the changes of the events, the history and the notifications that name them only by those
/// words: the job scrubs them a page of each table at a time, right after the change, without
/// undue delay (GDPR Art. 17(1)). A job that gives up is tried again every day, and the status page
/// counts the removal until it is finished.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct PendingRemoval {
    /// The person who is being removed.
    #[personal(category = "identity", basis = "legal_obligation")]
    #[person(subject)]
    pub person: UserId,
    /// Whether the person's comments go (the choice of the removal); they are hidden from the
    /// change of the removal on, and deleted by the job.
    #[not_personal]
    pub comments_go: bool,
    /// The job that does the next step; only this job may do it, so a step that runs twice never
    /// queues a second chain.
    #[not_personal]
    pub job: JobId,
    /// The next step, as the payload of the job writes it: IDs and a place in a table, no word of
    /// the person.
    #[personal(category = "technical", basis = "legal_obligation")]
    pub step: String,
    /// When the person was removed.
    #[personal(category = "activity", basis = "legal_obligation")]
    pub created_at: Timestamp,
    /// When the last step ended.
    #[not_personal]
    pub updated_at: Timestamp,
}

/// Every kind a removal deletes rows of, each with what the person is to a row that goes.
pub fn deleted_kinds() -> impl Iterator<Item = (EntityMarks, &'static [Link])> {
    DELETED.iter().copied()
}

/// Whether `kind` is the files of *Download my data*: the plan names them in words of their own,
/// so an administrator gives a file that was not fetched yet to the person before it goes.
#[must_use]
pub fn is_data_export(kind: &EntityMarks) -> bool {
    kind.name() == marks::<crate::data_export::DataExport>().name()
}

/// Whether rows of `kind` stay when they are shared with others: a saved search or a dashboard
/// shared with a group or a project is no longer the person's alone, so the removal keeps it,
/// shown with *Former user* as its maker (M5-F62, ADR-0050, decision 11); a private one, and the
/// person's *Home*, go with the account.
#[must_use]
pub fn kept_when_shared(kind: &EntityMarks) -> bool {
    [
        marks::<crate::saved_search::SavedSearch>(),
        marks::<crate::dashboard::Dashboard>(),
    ]
    .iter()
    .any(|shared| shared.name() == kind.name())
}

/// The kinds the law keeps for their period: work days, whose periods and breaks belong to them
/// (`ArbZG` §16, ADR-0035, decision 7), and the security log.
const KEPT_BY_LAW: &[(EntityMarks, KeepKind)] = &[
    (
        marks::<crate::working_time::record::WorkDay>(),
        KeepKind::WorkDays,
    ),
    (
        marks::<crate::security_log::SecurityLogEntry>(),
        KeepKind::SecurityLog,
    ),
];

/// What a removal does with the records of the entity of `kind`.
#[must_use]
pub fn at_removal(kind: &EntityMarks) -> AtRemoval {
    if let Some((_, by)) = DELETED
        .iter()
        .find(|(deleted, _)| deleted.name() == kind.name())
    {
        return AtRemoval::Deleted { by };
    }
    KEPT_BY_LAW
        .iter()
        .find(|(kept, _)| kept.name() == kind.name())
        .map_or(AtRemoval::KeptAsFormerUser, |(_, period)| {
            AtRemoval::KeptByLaw(*period)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priv_04_every_kind_named_by_the_removal_names_a_person_and_is_named_once() {
        let mut names: Vec<&str> = DELETED
            .iter()
            .map(|(marks, _)| marks)
            .chain(KEPT_BY_LAW.iter().map(|(marks, _)| marks))
            .map(EntityMarks::name)
            .collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "a kind is named twice");
        let nobody: Vec<&str> = DELETED
            .iter()
            .map(|(marks, _)| marks)
            .chain(KEPT_BY_LAW.iter().map(|(marks, _)| marks))
            .filter(|marks| marks.person_links().links().is_empty())
            .map(EntityMarks::name)
            .collect();
        assert!(
            nobody.is_empty(),
            "these name nobody, so the walk never finds them: {nobody:?}"
        );
        // Every link a deleted kind goes by is a link the kind has.
        let unknown: Vec<&str> = DELETED
            .iter()
            .filter(|(marks, by)| {
                !by.iter().any(|link| {
                    marks
                        .person_links()
                        .links()
                        .iter()
                        .any(|has| has.link() == Some(*link))
                })
            })
            .map(|(marks, _)| marks.name())
            .collect();
        assert!(unknown.is_empty(), "no link of these goes: {unknown:?}");
    }

    #[test]
    fn priv_04_the_work_days_and_the_security_log_are_kept_by_law_and_comments_stay() {
        assert_eq!(
            at_removal(&marks::<crate::working_time::record::WorkDay>()),
            AtRemoval::KeptByLaw(KeepKind::WorkDays)
        );
        assert_eq!(
            at_removal(&marks::<crate::security_log::SecurityLogEntry>()),
            AtRemoval::KeptByLaw(KeepKind::SecurityLog)
        );
        assert_eq!(
            at_removal(&marks::<crate::comment::Comment>()),
            AtRemoval::KeptAsFormerUser
        );
        assert_eq!(
            at_removal(&marks::<crate::sign_in::PasswordLogin>()),
            AtRemoval::Deleted { by: HOLDER }
        );
        // A notification goes with the person it was for, not with the person who caused it.
        assert_eq!(
            at_removal(&marks::<crate::notification::Notification>()),
            AtRemoval::Deleted { by: RECIPIENT }
        );
    }
}
