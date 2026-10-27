//! Issues, issue types and status groups (ISS-01, ISS-02, ADR-0027 decision 5).
//!
//! An issue has an internal ID that never changes and a readable ID (`readable_id.rs`) that can.
//! Its status is the key of a status of its workflow (`workflow.rs`), and every status belongs to a
//! status group. Entering the group *Done* sets the closed date; leaving it clears it.

use jiff::Timestamp;

use crate::id::{IssueId, IssueTypeId, ProjectId, UserId};
use crate::message::{self, MessageKey};
use crate::personal::Entity;
use crate::readable_id::IssueCode;
use crate::workflow::StatusKey;

/// The most people an issue is assigned to.
pub const ASSIGNED_PEOPLE_MAX: usize = 20;

words! {
    /// The group a status belongs to. Searches, boards and progress ask for the group, so a
    /// workflow can name its statuses as it likes.
    pub enum StatusGroup {
        /// Work that nobody started.
        NotStarted => "not-started",
        /// Work that is going on.
        InProgress => "in-progress",
        /// Work that is finished.
        Done => "done",
    }
    messages name = "status-group."
}

words_in_api!(StatusGroup, "The group of a status.");

impl StatusGroup {
    /// The order of the groups of a list: what is going on first, then what waits, then what is
    /// finished (the glossary names the same order).
    pub const LIST_ORDER: [Self; 3] = [Self::InProgress, Self::NotStarted, Self::Done];

    /// The word of the group in the search language: its code in lower camel case, the same in
    /// every language of the app (ADR-0005 amendment M5, decision 9).
    #[must_use]
    pub fn search_word(self) -> String {
        crate::fields::code_word(self.code())
    }

    /// The closed date after an issue enters a status of this group, given the date it had: the
    /// group *Done* sets it (an issue that is in the group already keeps the date it had), every
    /// other group has none (ADR-0027 decision 5, ADR-0031 decision 5).
    #[must_use]
    pub fn closed_after(self, closed_at: Option<Timestamp>, now: Timestamp) -> Option<Timestamp> {
        (self == Self::Done).then(|| closed_at.unwrap_or(now))
    }
}

words! {
    /// How important an issue is, from the least to the most.
    pub enum Priority {
        /// It can wait.
        Low => "low",
        /// The usual.
        Normal => "normal",
        /// It should come soon.
        High => "high",
        /// It cannot wait.
        Critical => "critical",
    }
    messages name = "priority."
}

words_in_api!(Priority, "How important an issue is.");

impl Priority {
    /// The word of the priority in the search language: its code in lower camel case, the same in
    /// every language of the app (ADR-0005 amendment M5, decision 9).
    #[must_use]
    pub fn search_word(self) -> String {
        crate::fields::code_word(self.code())
    }

    /// The priority of a new issue.
    pub const NEW: Self = Self::Normal;

    /// The order of the groups of a list: the most important first.
    pub const LIST_ORDER: [Self; 4] = [Self::Critical, Self::High, Self::Normal, Self::Low];
}

words! {
    /// The icon of an issue type: one of a fixed list of Lucide icons (UX §7.5), so every icon
    /// that can be stored is one the app ships. The word is the kebab-case name in Lucide.
    pub enum IssueIcon {
        /// A bug.
        Bug => "bug",
        /// A task that is ticked off.
        SquareCheck => "square-check",
        /// Something new.
        Sparkles => "sparkles",
        /// A goal.
        Flag => "flag",
        /// A target.
        Target => "target",
        /// An idea.
        Lightbulb => "lightbulb",
        /// A launch.
        Rocket => "rocket",
        /// A repair.
        Wrench => "wrench",
        /// Reading and writing.
        BookOpen => "book-open",
        /// Safety.
        Shield => "shield",
        /// Speed.
        Zap => "zap",
        /// A question.
        CircleHelp => "circle-help",
        /// Something that matters.
        Star => "star",
        /// Something to keep.
        Bookmark => "bookmark",
        /// Building.
        Hammer => "hammer",
        /// Heat.
        Flame => "flame",
    }
    messages name = "issue-icon."
}

words_in_api!(
    IssueIcon,
    "The icon of an issue type, by its name in Lucide."
);

/// An issue type that is made at the first start (ISS-02): its name is the message, so it is
/// written in the language of the catalog and can then be renamed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartingIssueType {
    /// The message that is the name.
    pub name: MessageKey,
    /// Its icon.
    pub icon: IssueIcon,
}

/// The issue types of the first start, in order: *Bug, Task, Feature, Goal*. The one definition;
/// after the first start the types are rows that can be changed, and this list is not read again.
pub const STARTING_ISSUE_TYPES: [StartingIssueType; 4] = [
    StartingIssueType {
        name: message::ISSUE_TYPE_BUG,
        icon: IssueIcon::Bug,
    },
    StartingIssueType {
        name: message::ISSUE_TYPE_TASK,
        icon: IssueIcon::SquareCheck,
    },
    StartingIssueType {
        name: message::ISSUE_TYPE_FEATURE,
        icon: IssueIcon::Sparkles,
    },
    StartingIssueType {
        name: message::ISSUE_TYPE_GOAL,
        icon: IssueIcon::Flag,
    },
];

/// The most issue types one place has: the whole organization, and each project, counted on its
/// own (the types created there).
pub const ISSUE_TYPES_MAX: usize = 100;

/// A kind of issue (ISS-02): *Bug*, *Task*, *Feature*, *Goal*, or one the organization or a project
/// made. Where it is offered is data (`offer`).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct IssueType {
    /// The ID of the type.
    #[not_personal]
    pub id: IssueTypeId,
    /// What it is called. Free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub name: String,
    /// Its icon.
    #[not_personal]
    pub icon: IssueIcon,
    /// The project it was made in (offered there and below until a place says otherwise, and
    /// looked after by whoever manages issue types there); none for the whole organization.
    #[not_personal]
    pub project: Option<ProjectId>,
    /// The place in the order the types are offered in.
    #[not_personal]
    pub position: i64,
    /// When it was retired: no new issues of this type, while the old ones keep it.
    #[not_personal]
    pub retired_at: Option<Timestamp>,
    /// Counts up with every change, for the check that nobody changed it meanwhile (ADR-0019).
    #[not_personal]
    pub version: i64,
}

/// An issue (ISS-01): a bug, a task, an idea, anything that is tracked.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Issue {
    /// The internal ID, which never changes.
    #[not_personal]
    pub id: IssueId,
    /// The project it is in.
    #[not_personal]
    pub project: ProjectId,
    /// Its issue type.
    #[not_personal]
    pub issue_type: IssueTypeId,
    /// The code of its readable ID now; the short name of its project comes before it.
    #[not_personal]
    pub code: IssueCode,
    /// What it is about. Free text that could name a person.
    #[personal(category = "content", basis = "contract")]
    pub title: String,
    /// The long text, in Markdown. Free text that could name a person.
    #[personal(category = "content", basis = "contract")]
    pub description: String,
    /// The key of its status in the workflow that counts for it.
    #[not_personal]
    pub status: StatusKey,
    /// How important it is.
    #[not_personal]
    pub priority: Priority,
    /// Who made it.
    #[personal(category = "activity", basis = "contract")]
    pub reported_by: UserId,
    /// When it was made.
    #[not_personal]
    pub created_at: Timestamp,
    /// When it was last changed.
    #[not_personal]
    pub updated_at: Timestamp,
    /// When it entered the status group *Done*, while it is in it.
    #[not_personal]
    pub closed_at: Option<Timestamp>,
    /// When it was deleted into the trash, while it is there.
    #[not_personal]
    pub deleted_at: Option<Timestamp>,
    /// Counts up with every change, for the check that nobody changed it meanwhile (ADR-0019).
    #[not_personal]
    pub version: i64,
}

/// One person an issue is assigned to; an issue can be assigned to several people.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct AssignedPerson {
    /// The issue.
    #[not_personal]
    pub issue: IssueId,
    /// Who works on it.
    #[personal(category = "activity", basis = "contract")]
    pub person: UserId,
    /// When the person was assigned.
    #[not_personal]
    pub added_at: Timestamp,
}

/// An old code of an issue (ADR-0021): the issue got a new readable ID (the mode of the project
/// changed, or it moved), and the old one keeps opening it. Nobody else can take the code: it is
/// unique in its project together with the current codes.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct IssueAlias {
    /// The project the code was in. That project's short name, now or an old one, starts the old
    /// issue ID.
    #[not_personal]
    pub project: ProjectId,
    /// The old code.
    #[not_personal]
    pub code: IssueCode,
    /// The issue it opens.
    #[not_personal]
    pub issue: IssueId,
    /// When the issue stopped using it.
    #[not_personal]
    pub retired_at: Timestamp,
}

/// The number of the last counting code a project gave out (ADR-0021). The only counter in the
/// product: the next issue of a project that counts has the number after it, and a number is
/// never given out twice, not even after the mode changed and changed back.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct IssueCounter {
    /// The project.
    #[not_personal]
    pub project: ProjectId,
    /// The last number given out.
    #[not_personal]
    pub last: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qry_05_the_search_words_of_priorities_and_status_groups_are_one_word_each_and_different() {
        let mut words: Vec<String> = Priority::ALL
            .iter()
            .map(|each| each.search_word())
            .collect();
        words.extend(StatusGroup::ALL.iter().map(|each| each.search_word()));
        let all = words.len();
        assert!(
            words
                .iter()
                .all(|word| word.chars().all(char::is_alphanumeric))
        );
        words.sort_unstable();
        words.dedup();
        assert_eq!(words.len(), all);
    }

    fn at(second: i64) -> Timestamp {
        Timestamp::from_second(second).unwrap()
    }

    #[test]
    fn adr_0027_the_closed_date_follows_the_done_group() {
        let (opened, closed) = (at(1), at(5));
        // Entering Done sets it; moving inside the group keeps the old date; every other group
        // has none.
        assert_eq!(StatusGroup::Done.closed_after(None, closed), Some(closed));
        assert_eq!(
            StatusGroup::Done.closed_after(Some(opened), closed),
            Some(opened)
        );
        assert_eq!(
            StatusGroup::InProgress.closed_after(Some(opened), closed),
            None
        );
        assert_eq!(StatusGroup::NotStarted.closed_after(None, closed), None);
    }

    #[test]
    fn iss_02_the_four_starting_types_are_bug_task_feature_and_goal_with_their_own_icons() {
        let names: Vec<&str> = STARTING_ISSUE_TYPES
            .iter()
            .map(|starting| starting.name.template())
            .collect();
        assert_eq!(names, ["Bug", "Task", "Feature", "Goal"]);
        let mut icons: Vec<IssueIcon> = STARTING_ISSUE_TYPES.iter().map(|t| t.icon).collect();
        icons.dedup();
        assert_eq!(icons.len(), STARTING_ISSUE_TYPES.len());
    }
}
