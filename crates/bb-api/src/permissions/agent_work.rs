//! What the routes of hand-overs, agent work and standing rules need (AI-31 to AI-35, ADR-0029):
//! the rows of the table of permissions for this area.
//!
//! Handing over and reviewing are judged by the use case on the issue the request names (*Change
//! issues* on each issue and *Create agents that work for you* in its project, M7-Q1); the standing
//! rules and their proposals are the person's own, like their agents. An agent, a key or an app
//! never reaches these routes: they start at a session.

use super::{Answer, Declared, Route, route};

/// The routes of agent work, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route("handOver", Declared::OwnAccount, Answer::Ordinary),
    route("getIssueAgentWork", Declared::ByRecord, Answer::NeverStored),
    route(
        "answerAgentQuestion",
        Declared::OwnAccount,
        Answer::Ordinary,
    ),
    route("approveAgentPlan", Declared::OwnAccount, Answer::Ordinary),
    route("reviewAgentWork", Declared::OwnAccount, Answer::Ordinary),
    route("lookAtAgentWork", Declared::OwnAccount, Answer::Ordinary),
    route("createStandingRule", Declared::OwnAccount, Answer::Ordinary),
    route("changeStandingRule", Declared::OwnAccount, Answer::Ordinary),
    route(
        "listStandingRules",
        Declared::OwnAccount,
        Answer::NeverStored,
    ),
    route("decideProposal", Declared::OwnAccount, Answer::Ordinary),
];
