//! The tables of agent work: hand-overs, standing rules, the work on an issue and what belongs to
//! it (AI-31 to AI-35, ADR-0029).

use super::{Iden, StoredTable};

/// Hand-overs of issues to an agent.
#[derive(Clone, Copy, Iden)]
pub(crate) enum HandOverRow {
    #[iden = "hand_over"]
    Table,
    Id,
    Agent,
    Person,
    Note,
    Powers,
    Fields,
    BudgetAttempts,
    BudgetMinutes,
    BudgetCost,
    PlanNeedsApproval,
    AcceptByItself,
    State,
    EndsAt,
    MadeAt,
    Version,
}

/// Standing rules of agents.
#[derive(Clone, Copy, Iden)]
pub(crate) enum StandingRuleRow {
    #[iden = "standing_rule"]
    Table,
    Id,
    Agent,
    Person,
    Search,
    Project,
    Fields,
    CreatesIn,
    State,
    EndsAt,
    MadeAt,
    Version,
}

/// The agent work on one issue.
#[derive(Clone, Copy, Iden)]
pub(crate) enum IssueWorkRow {
    #[iden = "issue_work"]
    Table,
    Id,
    Issue,
    HandOver,
    Agent,
    Person,
    State,
    GaveUp,
    Closed,
    AcceptedByItself,
    MadeAt,
    Version,
}

/// The attempts of an agent.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AttemptRow {
    #[iden = "agent_attempt"]
    Table,
    Id,
    Work,
    Number,
    Plan,
    PlanApproved,
    ReportedMinutes,
    ReportedCost,
    StartedAt,
    EndedAt,
}

/// What an attempt attached: links and checks.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AttemptResultRow {
    #[iden = "agent_result"]
    Table,
    Id,
    Attempt,
    Kind,
    Label,
    Link,
    Outcome,
}

/// The questions of an agent with their answers.
#[derive(Clone, Copy, Iden)]
pub(crate) enum QuestionRow {
    #[iden = "agent_question"]
    Table,
    Id,
    Work,
    Text,
    Answer,
    AnsweredBy,
    AskedAt,
}

/// The reviews of agent work.
#[derive(Clone, Copy, Iden)]
pub(crate) enum ReviewRow {
    #[iden = "agent_review"]
    Table,
    Id,
    Work,
    Answer,
    Note,
    By,
    At,
}

/// What standing rules propose.
#[derive(Clone, Copy, Iden)]
pub(crate) enum ProposalRow {
    #[iden = "agent_proposal"]
    Table,
    Id,
    Rule,
    Agent,
    Issue,
    Field,
    Before,
    After,
    State,
    DecidedBy,
    MadeAt,
}

/// The issues that came under a standing rule, with the moment.
#[derive(Clone, Copy, Iden)]
pub(crate) enum RuleMatchRow {
    #[iden = "rule_match"]
    Table,
    Rule,
    Issue,
    At,
}

stored_tables! {
        HandOverRow => bb_core::agent_records::HandOver {
        Id => id,
        Agent => agent,
        Person => person,
        Note => note,
        Powers => powers,
        Fields => fields,
        BudgetAttempts => budget_attempts,
        BudgetMinutes => budget_minutes,
        BudgetCost => budget_cost,
        PlanNeedsApproval => plan_needs_approval,
        AcceptByItself => accept_by_itself,
        State => state,
        EndsAt => ends_at,
        MadeAt => made_at,
        Version => version,
    },
    StandingRuleRow => bb_core::agent_records::StandingRule {
        Id => id,
        Agent => agent,
        Person => person,
        Search => search,
        Project => project,
        Fields => fields,
        CreatesIn => creates_in,
        State => state,
        EndsAt => ends_at,
        MadeAt => made_at,
        Version => version,
    },
    IssueWorkRow => bb_core::agent_records::IssueWork {
        Id => id,
        Issue => issue,
        HandOver => hand_over,
        Agent => agent,
        Person => person,
        State => state,
        GaveUp => gave_up,
        Closed => closed,
        AcceptedByItself => accepted_by_itself,
        MadeAt => made_at,
        Version => version,
    },
    AttemptRow => bb_core::agent_records::Attempt {
        Id => id,
        Work => work,
        Number => number,
        Plan => plan,
        PlanApproved => plan_approved,
        ReportedMinutes => reported_minutes,
        ReportedCost => reported_cost,
        StartedAt => started_at,
        EndedAt => ended_at,
    },
    AttemptResultRow => bb_core::agent_records::AttemptResult {
        Id => id,
        Attempt => attempt,
        Kind => kind,
        Label => label,
        Link => link,
        Outcome => outcome,
    },
    QuestionRow => bb_core::agent_records::Question {
        Id => id,
        Work => work,
        Text => text,
        Answer => answer,
        AnsweredBy => answered_by,
        AskedAt => asked_at,
    },
    ReviewRow => bb_core::agent_records::Review {
        Id => id,
        Work => work,
        Answer => answer,
        Note => note,
        By => by,
        At => at,
    },
    ProposalRow => bb_core::agent_records::Proposal {
        Id => id,
        Rule => rule,
        Agent => agent,
        Issue => issue,
        Field => field,
        Before => before,
        After => after,
        State => state,
        DecidedBy => decided_by,
        MadeAt => made_at,
    },
    RuleMatchRow => bb_core::agent_records::RuleMatch {
        Rule => rule,
        Issue => issue,
        At => at,
    },
}
