//! What the routes of agents need (AI-30, ADR-0029): the rows of the table of permissions for this
//! area, kept beside it so the table does not grow.
//!
//! A person's own agents are the person's own account, and so are their keys, which only a session
//! makes. An agent key reads the agent's own record (`getAgent`) and nothing else; every other
//! route starts at a session. Making an agent is judged by the use case: the person holds *Create
//! agents that work for you* in at least one project (M7-Q1), and the route names no project.

use bb_policy::Permission;

use super::{Answer, Declared, Route, route};

/// The routes of agents, by operation ID.
pub(super) const ROUTES: &[Route] = &[
    route("listMyAgents", Declared::OwnAccount, Answer::NeverStored),
    route("createAgent", Declared::OwnAccount, Answer::Ordinary),
    route("getAgent", Declared::SignedIn, Answer::NeverStored),
    route("changeAgent", Declared::OwnAccount, Answer::Ordinary),
    route("handAgent", Declared::OwnAccount, Answer::Ordinary),
    route("acceptAgent", Declared::OwnAccount, Answer::Ordinary),
    route("takeBackAgentOffer", Declared::OwnAccount, Answer::Ordinary),
    route("createAgentKey", Declared::OwnAccount, Answer::Secret),
    route("renewAgentKey", Declared::OwnAccount, Answer::Secret),
    route("deleteAgentKey", Declared::OwnAccount, Answer::Ordinary),
    route("stopAllMyAgents", Declared::OwnAccount, Answer::Ordinary),
    route(
        "listOrganizationAgents",
        Declared::Permission(Permission::ChangeOrganizationSettings),
        Answer::NeverStored,
    ),
];
