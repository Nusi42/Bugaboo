//! Agent work (AI-31 to AI-35, ADR-0029): the rules of hand-overs, standing rules and the review,
//! written once and free of storage.
//!
//! A person gives an agent work in two ways only: a hand-over of issues by their IDs, or a standing
//! rule over a saved search with narrow powers. What the agent does on an issue is its own record.
//! The functions here answer the questions both the use cases and the jobs ask: when does it end,
//! may a rule change this field, what is missing in the brief, is an attempt accepted by itself.

use jiff::{SignedDuration, Timestamp};

use crate::settings::{AGENTS_HAND_OVER_DAYS, AGENTS_RULE_DAYS, DefaultValue, SettingDefinition};

/// The default of a whole-number setting, or 0 for a setting that is none.
const fn default_days(definition: &SettingDefinition) -> i64 {
    match definition.default() {
        DefaultValue::Integer(days) => days,
        _ => 0,
    }
}

/// The largest value of a bounded whole-number setting, or 0 for one that is not.
const fn most_days(definition: &SettingDefinition) -> i64 {
    match definition.bounds() {
        Some((_, most)) => most,
        None => 0,
    }
}

/// A hand-over ends after this many days unless the person set another end (ADR-0029, decision
/// 3); the number is the default of the setting `agents.hand-over-days`, written there once.
pub const DEFAULT_HAND_OVER_DAYS: i64 = default_days(AGENTS_HAND_OVER_DAYS.definition());
/// The longest a hand-over can last, so no work stays open for good (the bound of the setting).
pub const MOST_HAND_OVER_DAYS: i64 = most_days(AGENTS_HAND_OVER_DAYS.definition());
/// A standing rule runs out after this many days unless set otherwise (the default of the setting
/// `agents.rule-days`).
pub const DEFAULT_RULE_DAYS: i64 = default_days(AGENTS_RULE_DAYS.definition());
/// The longest a standing rule can last, however often it is set or renewed (the bound of the
/// setting).
pub const MOST_RULE_DAYS: i64 = most_days(AGENTS_RULE_DAYS.definition());
/// The notification *A standing rule runs out soon* comes this many days before the end.
pub const RULE_WARNING_DAYS: i64 = 3;
/// The most issues one hand-over names.
pub const MOST_ISSUES_OF_A_HAND_OVER: usize = 100;

words! {
    /// The state of the agent work on an issue. It is not the issue's status.
    pub enum AgentWorkState {
        /// Handed over, and the agent has not taken it yet.
        WaitingForTheAgent => "waiting-for-the-agent",
        /// An attempt is running.
        AgentWorking => "agent-working",
        /// A question or a plan waits for a person.
        WaitingForYou => "waiting-for-you",
        /// An attempt is ready and waits for the review.
        ReadyForReview => "ready-for-review",
        /// The agent gave up, with a reason.
        AgentGaveUp => "agent-gave-up",
        /// The agent was stopped, its person lost access, or the hand-over ran out.
        Stopped => "stopped",
    }
}

crate::words_in_api!(AgentWorkState, "The state of the agent work on an issue.");

words! {
    /// What a hand-over lets the agent do on top of what it always may (read the issue and what it
    /// links to, comment, report, attach results, link related issues).
    pub enum HandOverPower {
        /// Change the fields the hand-over names.
        ChangeFields => "change-fields",
        /// Change the status.
        ChangeStatus => "change-status",
        /// Create sub-issues.
        CreateSubIssues => "create-sub-issues",
    }
}

crate::words_in_api!(
    HandOverPower,
    "What a hand-over lets the agent do besides what it always may."
);

words! {
    /// The fields a standing rule may be told to change. Never a field that holds people, never the
    /// status: the list is closed, so there is no word for them (COMP-10).
    pub enum RuleField {
        /// Labels.
        Labels => "labels",
        /// Tags.
        Tags => "tags",
        /// The type of the issue.
        Type => "type",
        /// Duplicate of.
        DuplicateOf => "duplicate-of",
        /// Found in release.
        FoundInRelease => "found-in-release",
    }
}

crate::words_in_api!(RuleField, "A field a standing rule may change.");

words! {
    /// The answers of a review.
    pub enum ReviewAnswer {
        /// Keep the result; the hand-over ends.
        Accept => "accept",
        /// Try again; the note joins the brief.
        SendBack => "send-back",
        /// Undo everything the attempts changed.
        ThrowAway => "throw-away",
    }
}

crate::words_in_api!(ReviewAnswer, "How a person answers a result at the review.");

words! {
    /// The result of one check an agent attached (M7-Q8). Bugaboo does not run or verify checks.
    pub enum CheckResult {
        /// The check passed.
        Passed => "passed",
        /// The check failed.
        Failed => "failed",
    }
}

crate::words_in_api!(
    CheckResult,
    "The result of a check an agent attached to an attempt."
);

words! {
    /// The parts of a brief that a hand-over takes from the issue. What is missing is named, and
    /// never stops the hand-over.
    pub enum BriefPart {
        /// The description.
        Description => "description",
        /// *Steps to reproduce*.
        StepsToReproduce => "steps-to-reproduce",
        /// *Done when*.
        DoneWhen => "done-when",
    }
}

crate::words_in_api!(BriefPart, "A part of a brief taken from the issue.");

/// The parts of the brief that the issue leaves empty, in the order of [`BriefPart`]. A part the
/// reader may not see (`None`) is not named: whether it is empty is not theirs to learn.
#[must_use]
pub fn missing_in_brief(
    description: &str,
    steps: Option<&str>,
    done_when: Option<&str>,
) -> Vec<BriefPart> {
    [
        (BriefPart::Description, Some(description)),
        (BriefPart::StepsToReproduce, steps),
        (BriefPart::DoneWhen, done_when),
    ]
    .into_iter()
    .filter(|(_, text)| text.is_some_and(|text| text.trim().is_empty()))
    .map(|(part, _)| part)
    .collect()
}

/// Why an end cannot be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndError {
    /// The end is not after the start, or is further away than the longest.
    OutOfRange,
}

/// The end of something that lasts `days` from `start`, for `days` of `1..=most`; `None` takes the
/// default.
fn end_after(
    start: Timestamp,
    days: Option<i64>,
    default: i64,
    most: i64,
) -> Result<Timestamp, EndError> {
    let days = days.unwrap_or(default);
    if !(1..=most).contains(&days) {
        return Err(EndError::OutOfRange);
    }
    start
        .checked_add(SignedDuration::from_hours(24 * days))
        .map_err(|_| EndError::OutOfRange)
}

/// The end of a hand-over made at `start`: 14 days unless set, at most 90.
///
/// # Errors
/// [`EndError::OutOfRange`] for fewer than one day or more than the most.
pub fn hand_over_end(start: Timestamp, days: Option<i64>) -> Result<Timestamp, EndError> {
    end_after(start, days, DEFAULT_HAND_OVER_DAYS, MOST_HAND_OVER_DAYS)
}

/// The end of a standing rule made or renewed at `start`: 30 days unless set, never more than 90.
///
/// # Errors
/// [`EndError::OutOfRange`] for fewer than one day or more than 90.
pub fn rule_end(start: Timestamp, days: Option<i64>) -> Result<Timestamp, EndError> {
    end_after(start, days, DEFAULT_RULE_DAYS, MOST_RULE_DAYS)
}

/// When the warning that a rule runs out comes: three days before its end.
#[must_use]
pub fn rule_warning_at(end: Timestamp) -> Option<Timestamp> {
    end.checked_sub(SignedDuration::from_hours(24 * RULE_WARNING_DAYS))
        .ok()
}

/// What a standing rule may do (AI-32). `Propose` is the start: nothing changes until a person
/// applies it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RulePower {
    /// Write proposals only.
    Propose,
    /// Change only these fields.
    ChangeFields(Vec<RuleField>),
}

impl RulePower {
    /// Whether the rule may change `field` by itself.
    #[must_use]
    pub fn may_change(&self, field: RuleField) -> bool {
        matches!(self, Self::ChangeFields(fields) if fields.contains(&field))
    }
}

/// The members of an issue an agent can be given to change, by their keys: the word of the field
/// in the catalogue (`Field::code`) as events and levels name it.
pub mod issue_member {
    use crate::fields::Field;

    /// The issue type.
    pub const ISSUE_TYPE: &str = "issue-type";
    /// The title.
    pub const TITLE: &str = Field::Title.code();
    /// The description.
    pub const DESCRIPTION: &str = Field::Description.code();
    /// The status: only when the hand-over ticked it.
    pub const STATUS: &str = Field::Status.code();
    /// The priority.
    pub const PRIORITY: &str = Field::Priority.code();
    /// *Estimated time*.
    pub const ESTIMATED_TIME: &str = Field::EstimatedTime.code();
    /// *Time left*.
    pub const TIME_LEFT: &str = Field::TimeLeft.code();
    /// The labels.
    pub const LABELS: &str = Field::Labels.code();
    /// The tags.
    pub const TAGS: &str = Field::Tags.code();
    /// The relations of an issue: an agent links related issues without being given anything, and
    /// takes a link away only when it was given this.
    pub const RELATIONS: &str = "relations";
    /// The comments of an issue: an agent adds its own, and edits, deletes or restores one only
    /// when it was given this.
    pub const COMMENTS: &str = "comments";
    /// The attachments of an issue: an agent attaches results, and deletes or restores a file only
    /// when it was given this.
    pub const ATTACHMENTS: &str = "attachments";
    /// *Assigned to*: holds people.
    pub const ASSIGNED_TO: &str = Field::AssignedTo.code();
    /// *Reported by*: holds people.
    pub const REPORTED_BY: &str = Field::ReportedBy.code();
    /// The followers of an issue: hold people.
    pub const FOLLOWERS: &str = "followers";

    /// The members a hand-over can name next to the own fields of the catalogue; the status is
    /// ticked as a power, never named.
    pub const NAMEABLE: [&str; 11] = [
        TITLE,
        DESCRIPTION,
        PRIORITY,
        ESTIMATED_TIME,
        TIME_LEFT,
        ISSUE_TYPE,
        LABELS,
        TAGS,
        RELATIONS,
        COMMENTS,
        ATTACHMENTS,
    ];

    /// The members that hold people. An agent never changes one of them, whatever it was given
    /// (ADR-0029, decision 3; COMP-10).
    pub const PEOPLE: [&str; 3] = [ASSIGNED_TO, FOLLOWERS, REPORTED_BY];
}

/// What an agent may change on one issue, from the hand-overs and standing rules it works under
/// (ADR-0029, decision 3). Reading, commenting, reporting and attaching are always allowed and are
/// not part of it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Allowance {
    /// The members of the issue it may change, by their keys.
    pub fields: Vec<String>,
    /// Whether it may change the status.
    pub status: bool,
    /// Whether it may create sub-issues.
    pub sub_issues: bool,
}

impl Allowance {
    /// The allowance a hand-over gives: its ticks and, when it may change fields, the named ones.
    #[must_use]
    pub fn of_hand_over(powers: &[HandOverPower], fields: &[String]) -> Self {
        Self {
            fields: if powers.contains(&HandOverPower::ChangeFields) {
                fields.to_vec()
            } else {
                Vec::new()
            },
            status: powers.contains(&HandOverPower::ChangeStatus),
            sub_issues: powers.contains(&HandOverPower::CreateSubIssues),
        }
    }

    /// The allowance a standing rule gives: only the fields it names, and only those that are
    /// members of an issue.
    #[must_use]
    pub fn of_rule(fields: &[RuleField]) -> Self {
        Self {
            fields: fields
                .iter()
                .filter_map(|field| field.issue_member())
                .map(str::to_owned)
                .collect(),
            ..Self::default()
        }
    }

    /// Adds what another hand-over or rule allows on the same issue.
    pub fn add(&mut self, other: Self) {
        for field in other.fields {
            if !self.fields.contains(&field) {
                self.fields.push(field);
            }
        }
        self.status |= other.status;
        self.sub_issues |= other.sub_issues;
    }

    /// Whether the agent may change this member of the issue: never one that holds people, and
    /// the status only when it was ticked.
    #[must_use]
    pub fn may_change(&self, member: &str, status_member: &str) -> bool {
        if issue_member::PEOPLE.contains(&member) {
            return false;
        }
        if member == status_member {
            return self.status;
        }
        self.fields.iter().any(|field| field == member)
    }
}

impl RuleField {
    /// The member of an issue that a change of this field writes, when it has one. *Duplicate of*
    /// and *Found in release* are changed through a relation and an own field, which an agent
    /// reaches only as the links of related issues and the fields a hand-over names.
    #[must_use]
    pub const fn issue_member(self) -> Option<&'static str> {
        match self {
            Self::Type => Some(issue_member::ISSUE_TYPE),
            Self::Labels => Some(issue_member::LABELS),
            Self::Tags => Some(issue_member::TAGS),
            Self::DuplicateOf | Self::FoundInRelease => None,
        }
    }
}

/// The state of agent work after the agent is stopped or its person loses access: open work is
/// *Stopped*, work that ended stays as it ended.
#[must_use]
pub fn after_stop(state: AgentWorkState) -> AgentWorkState {
    AgentWorkState::Stopped.or_keep_ended(state)
}

impl AgentWorkState {
    fn or_keep_ended(self, was: Self) -> Self {
        if Self::ENDED.contains(&was) {
            was
        } else {
            self
        }
    }

    /// The states that end work: the agent gave up, or it was stopped. Everything that asks whether
    /// work is open (use cases, statements, searches) asks through this.
    pub const ENDED: [Self; 2] = [Self::AgentGaveUp, Self::Stopped];

    /// Whether the work is still open (an attempt may start or go on).
    #[must_use]
    pub fn is_open(self) -> bool {
        !Self::ENDED.contains(&self)
    }

    /// The states in which an attempt is running.
    pub const IN_ATTEMPT: [Self; 2] = [Self::AgentWorking, Self::WaitingForYou];

    /// Whether an attempt is running in this state: the agent works, or has asked and waits. Of two
    /// pieces of work on one issue, only one is in an attempt at a time (ADR-0029, decision 9). The
    /// one place that says it.
    #[must_use]
    pub const fn in_attempt(self) -> bool {
        matches!(self, Self::AgentWorking | Self::WaitingForYou)
    }

    /// The state after a question: *Waiting for you*. None for work that cannot ask: work that
    /// ended, and a result that waits for its review (the one place that says so).
    #[must_use]
    pub fn after_question(self) -> Option<Self> {
        (self.is_open() && self != Self::ReadyForReview).then_some(Self::WaitingForYou)
    }
}

/// Whether an attempt is accepted by itself (M7-Q8): only when the project allows it, the
/// hand-over did not turn it off, the attempt has at least one check, and every check passed.
#[must_use]
pub fn accepts_by_itself(
    project_allows: bool,
    hand_over_allows: bool,
    checks: &[CheckResult],
) -> bool {
    project_allows
        && hand_over_allows
        && !checks.is_empty()
        && checks.iter().all(|check| *check == CheckResult::Passed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    #[test]
    fn ai_31_a_hand_over_runs_out_after_14_days() {
        let start = at("2026-11-01T10:00:00Z");
        assert_eq!(hand_over_end(start, None), Ok(at("2026-11-15T10:00:00Z")));
        assert_eq!(hand_over_end(start, Some(0)), Err(EndError::OutOfRange));
    }

    #[test]
    fn ai_31_the_brief_names_what_is_missing_and_does_not_stop_the_hand_over() {
        let missing = missing_in_brief("Crash on save", Some("  "), Some(""));
        assert_eq!(
            missing,
            vec![BriefPart::StepsToReproduce, BriefPart::DoneWhen]
        );
        assert!(missing_in_brief("a", Some("b"), Some("c")).is_empty());
        // A part the reader may not see is not named, empty or not.
        assert_eq!(
            missing_in_brief("a", None, Some("")),
            vec![BriefPart::DoneWhen]
        );
    }

    #[test]
    fn ai_32_a_standing_rule_runs_out_after_30_days_and_never_after_more_than_90() {
        let start = at("2026-11-01T00:00:00Z");
        assert_eq!(rule_end(start, None), Ok(at("2026-12-01T00:00:00Z")));
        assert!(rule_end(start, Some(MOST_RULE_DAYS)).is_ok());
        assert_eq!(rule_end(start, Some(91)), Err(EndError::OutOfRange));
        let end = at("2026-12-01T00:00:00Z");
        assert_eq!(rule_warning_at(end), Some(at("2026-11-28T00:00:00Z")));
    }

    #[test]
    fn ai_32_a_standing_rule_never_changes_people_fields() {
        // The list of fields is closed: no word names a field of people or the status.
        for field in RuleField::ALL {
            let code = field.code();
            for forbidden in ["assigned-to", "followers", "reported-by", "status"] {
                assert_ne!(code, forbidden);
            }
        }
        assert!(!RulePower::Propose.may_change(RuleField::Labels));
        assert!(RulePower::ChangeFields(vec![RuleField::Labels]).may_change(RuleField::Labels));
        assert!(!RulePower::ChangeFields(vec![RuleField::Labels]).may_change(RuleField::Tags));
    }

    #[test]
    fn ai_32_an_agent_never_changes_a_member_that_holds_people_or_the_status_unasked() {
        let hand_over = Allowance::of_hand_over(
            &[HandOverPower::ChangeFields],
            &[
                issue_member::TITLE.to_owned(),
                issue_member::ASSIGNED_TO.to_owned(),
            ],
        );
        assert!(hand_over.may_change("title", "status"));
        assert!(
            !hand_over.may_change("assigned-to", "status"),
            "people never"
        );
        assert!(!hand_over.may_change("status", "status"), "not ticked");
        assert!(!hand_over.may_change("priority", "status"), "not named");
        let status = Allowance::of_hand_over(&[HandOverPower::ChangeStatus], &[]);
        assert!(status.may_change("status", "status"));
        let rule = Allowance::of_rule(&[RuleField::Type, RuleField::Labels]);
        assert!(rule.may_change("issue-type", "status"));
        assert!(!rule.may_change("status", "status"));
    }

    #[test]
    fn ai_33_open_work_becomes_stopped_and_ended_work_stays() {
        assert_eq!(
            after_stop(AgentWorkState::AgentWorking),
            AgentWorkState::Stopped
        );
        assert_eq!(
            after_stop(AgentWorkState::WaitingForYou),
            AgentWorkState::Stopped
        );
        assert_eq!(
            after_stop(AgentWorkState::AgentGaveUp),
            AgentWorkState::AgentGaveUp
        );
    }

    #[test]
    fn ai_34_a_question_sets_waiting_for_you() {
        assert_eq!(
            AgentWorkState::AgentWorking.after_question(),
            Some(AgentWorkState::WaitingForYou)
        );
        assert_eq!(AgentWorkState::Stopped.after_question(), None);
        assert_eq!(AgentWorkState::ReadyForReview.after_question(), None);
    }

    #[test]
    fn ai_35_an_attempt_without_a_check_or_with_a_failed_one_waits_for_a_person() {
        use CheckResult::{Failed, Passed};
        assert!(accepts_by_itself(true, true, &[Passed, Passed]));
        assert!(!accepts_by_itself(true, true, &[]));
        assert!(!accepts_by_itself(true, true, &[Passed, Failed]));
        assert!(!accepts_by_itself(false, true, &[Passed]));
        assert!(!accepts_by_itself(true, false, &[Passed]));
    }
}
