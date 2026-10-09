//! Every route for every actor (SEC-MATRIX, SEC-01, SEC-02, SEC-03, SEC-06, ADR-0004, ADR-0025).
//!
//! The test reads the API description, takes the route and the permission it declares from it, and
//! runs the example request of the route (`examples.rs`) as every actor of the fixed list
//! (`actors.rs`) against the real server (the router, the use cases, the store; SQLite and
//! PostgreSQL). What each actor may do is written down in `actors.rs` by hand from the tables of
//! the documents; the answer of the server is compared with it:
//!
//! - *allowed*: the request is let in (what comes then is the business of the route);
//! - *refused*: nobody signed in is `auth.not_signed_in`; a person who lacks the permission is
//!   `access.forbidden`, but where they cannot know the project exists, `resource.not_found`, the
//!   very answer of a project that does not exist.
//!
//! The record of what ran (`tools/checks/sec-matrix-record.txt`) is compared with the file in the
//! repository, which SEC-MATRIX (xtask) holds against the API description: a route that is missing
//! from it fails the check.

use std::cell::Cell;
use std::collections::BTreeMap;

use axum::Router;
use axum::http::Method;
use axum::http::StatusCode;
use bb_api::openapi::{Operation, operations};
use bb_api::permissions::Declared;
use bb_app::access::{Permission, StartingRole};
use bb_app::auth::MakeAccessKey;
use bb_app::roles::RolesQuery;
use bb_core::access_key::KeyChoice;
use bb_core::id::{ProjectId, RoleId};
use bb_testing::{Failure, TestResult, each_database};

use super::actors::{
    Actor, Project, Where, actors, everything_seen, held_exactly, short_name, standing,
    top_of_the_tree,
};
use super::examples::{EXAMPLES, Example, Quirk, request};
use super::record::Record;
use crate::sign_in_tests::http::{Browser, Reply, called};
use crate::sign_in_tests::policy_rig::{Member, Org};
use crate::sign_in_tests::rig::{PASSWORD, Rig, Tries};
use crate::sign_in_tests::status::app_with_probe;

/// What the server must answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Expect {
    /// The request is let in.
    Allowed,
    /// Nobody is signed in: `auth.not_signed_in`.
    NotSignedIn,
    /// The actor knows the place and lacks the permission: `access.forbidden`.
    Forbidden,
    /// The actor may not know about the project: `resource.not_found`.
    NotFound,
    /// An access key at a route of the owner's own sign-in data: `auth.access_key_not_allowed`.
    NotForKeys,
}

/// What the server must answer `actor` for `route`, from the table of `actors.rs`.
pub(super) fn expectation(actor: Actor, route: &Operation, example: &Example) -> Expect {
    if route.needs == Declared::Public {
        return Expect::Allowed;
    }
    // A key never reaches its owner's own sign-in data (ADR-0025, decision 8); `signed-in` is about
    // the session of a browser: who is signed in, signing out.
    if actor.has_a_key() && matches!(route.needs, Declared::SignedIn | Declared::OwnAccount) {
        if example.has(Quirk::KeysActAsThePerson) {
            // The choices of the person: a key reads them and, if it may change, changes them.
            return if route.method == Method::GET || actor.key_may_change() {
                Expect::Allowed
            } else {
                Expect::Forbidden
            };
        }
        return Expect::NotForKeys;
    }
    if !actor.signed_in() || example.has(Quirk::FinishesAHalfDoneSignIn) {
        return Expect::NotSignedIn;
    }
    let permission = match route.needs {
        Declared::Public | Declared::SignedIn | Declared::OwnAccount => return Expect::Allowed,
        Declared::Permission(permission) => Some(permission),
        // The use case judges the record the request names; the example names the target project.
        Declared::ByRecord => example.record_permission(),
    };
    match permission {
        None => Expect::Allowed,
        Some(permission) => {
            if example.has(Quirk::FiltersWhatIsSeen) {
                return Expect::Allowed;
            }
            let on_the_target = example.record_permission().is_some();
            let at = if route.path.contains("{project}")
                || example.has(Quirk::AtTheTargetProject)
                || on_the_target
            {
                Where::Target
            } else {
                Where::Organization
            };
            let held = standing(actor, at);
            // A record that is not seen is not found, before its permission is looked at.
            if at == Where::Target
                && (example.has(Quirk::HidesUnseen) || on_the_target)
                && !held.sees()
            {
                // What is inside a project is not told to a person who does not see it.
                Expect::NotFound
            } else if held.permissions.contains(&permission) {
                if at == Where::Target && example.has(Quirk::NeedsToSeeTheProject) && !held.sees() {
                    Expect::NotFound
                } else {
                    Expect::Allowed
                }
            } else if at == Where::Target
                && (!held.knows
                    || ((example.has(Quirk::HidesUnseen) || on_the_target) && !held.sees()))
            {
                Expect::NotFound
            } else {
                Expect::Forbidden
            }
        }
    }
}

/// The refusals the matrix knows how to expect.
const KNOWN_REFUSALS: [&str; 3] = [
    "auth.not_signed_in",
    "access.forbidden",
    "auth.access_key_not_allowed",
];

/// What a reply is, for the comparison.
fn seen(reply: &Reply) -> Expect {
    match reply.code() {
        "auth.not_signed_in" => Expect::NotSignedIn,
        "access.forbidden" => Expect::Forbidden,
        "resource.not_found" => Expect::NotFound,
        "auth.access_key_not_allowed" => Expect::NotForKeys,
        _ => Expect::Allowed,
    }
}

/// The server of the matrix: an organization with three projects and one person for each actor.
pub(super) struct Fixture<'a> {
    org: Org<'a>,
    app: Router,
    projects: BTreeMap<&'static str, ProjectId>,
    people: Vec<(Actor, String)>,
    /// The secrets of the access keys of the actors that come with one.
    keys: Vec<(Actor, String)>,
    /// The roles by their starting key, for the routes that name a role.
    roles: BTreeMap<String, RoleId>,
    numbers: Cell<u32>,
}

impl<'a> Fixture<'a> {
    /// The rig behind the fixture.
    pub(super) fn rig(&self) -> &Rig {
        self.org.rig
    }

    /// The session of the administrator who set the organization up, for what the matrix has to
    /// make before a call (an invitation to take back).
    pub(super) fn admin_session(&self) -> &str {
        &self.org.admin.session
    }

    /// The project of the matrix.
    pub(super) fn project(&self, project: Project) -> ProjectId {
        self.projects[short_name(project)]
    }

    /// A person of the organization, for the routes that name one: the administrator.
    pub(super) fn a_person(&self) -> bb_core::id::UserId {
        self.org.admin.id
    }

    /// A role of the organization, for the routes that name one.
    pub(super) fn a_role(&self) -> Result<RoleId, Failure> {
        self.roles
            .get(StartingRole::Viewer.code())
            .copied()
            .ok_or_else(|| "the organization has no viewer role".into())
    }

    /// The person who set the organization up.
    pub(super) fn admin_id(&self) -> bb_core::id::UserId {
        self.org.admin.id
    }

    /// A number that no other call of the matrix had.
    pub(super) fn next_number(&self) -> u32 {
        let number = self.numbers.get() + 1;
        self.numbers.set(number);
        number
    }

    /// The projects of the matrix, the person of every actor, and what they hold.
    async fn start(rig: &'a Rig) -> Result<Self, Failure> {
        let org = Org::start(rig).await?;
        let new_project = async |parent: Option<ProjectId>, project: Project| {
            org.project(parent, short_name(project))
                .await
                .map(|view| view.id)
        };
        let parent = new_project(None, Project::Parent).await?;
        let target = new_project(Some(parent), Project::Target).await?;
        let sibling = new_project(Some(parent), Project::Sibling).await?;
        let projects = BTreeMap::from([
            (short_name(Project::Parent), parent),
            (short_name(Project::Target), target),
            (short_name(Project::Sibling), sibling),
        ]);
        let roles = starting_roles(&org).await?;
        let places = [parent, target, sibling];
        let mut people = Vec::new();
        let mut keys = Vec::new();
        let mut everyone = actors();
        everyone.extend(
            Permission::ALL
                .iter()
                .map(|permission| Actor::Exactly(Some(*permission))),
        );
        everyone.push(Actor::Exactly(None));
        for actor in everyone {
            if !actor.signed_in() {
                continue;
            }
            let email = format!("{}@example.test", actor.key());
            let started = rig.person(&org.admin.session, &email).await?;
            let member = Member {
                id: started.person.id,
                session: started.session_token,
            };
            if let Some(secret) = hold(&org, rig, actor, &member, &places, &roles).await? {
                keys.push((actor, secret));
            }
            people.push((actor, email));
        }
        // With a probe, so the status page of the administrators answers.
        let app = app_with_probe(rig, None);
        Ok(Self {
            org,
            app,
            projects,
            people,
            keys,
            roles,
            numbers: Cell::new(0),
        })
    }

    /// A browser of the actor, signed in (nobody's is not).
    async fn browser(&self, actor: Actor) -> Result<Browser, Failure> {
        let mut browser = Browser::new(&self.app, &called());
        if let Some((_, secret)) = self.keys.iter().find(|(known, _)| *known == actor) {
            // A script: no cookie, the key in the header.
            browser.set_header("authorization", &format!("Bearer {secret}"));
            return Ok(browser);
        }
        if let Some((_, email)) = self.people.iter().find(|(known, _)| *known == actor) {
            let signed_in = browser.sign_in(email, PASSWORD).await?;
            if signed_in.status != StatusCode::OK {
                return Err(
                    format!("{} could not sign in: {}", actor.name(), signed_in.code()).into(),
                );
            }
        }
        Ok(browser)
    }
}

/// Gives the person of an actor what the actor holds: the roles and the *No access*, and for an
/// actor that comes with an access key, the key, which is made by its owner with the password asked
/// again. The secret of the key, if there is one.
async fn hold(
    org: &Org<'_>,
    rig: &Rig,
    actor: Actor,
    member: &Member,
    places: &[ProjectId; 3],
    roles: &BTreeMap<String, RoleId>,
) -> Result<Option<String>, Failure> {
    let [parent, target, sibling] = *places;
    let role = |role: StartingRole| roles[role.code()];
    match actor {
        Actor::Nobody | Actor::NoRole | Actor::Exactly(None) => {}
        Actor::Exactly(Some(permission)) => {
            let only = org
                .role(
                    &format!("Exactly {}", permission.code()),
                    &held_exactly(permission),
                )
                .await?;
            org.give_everywhere(member, only.id).await?;
        }
        Actor::Role(starting) => {
            org.give_everywhere(member, role(starting)).await?;
        }
        Actor::LeadOnParent => {
            org.give(member, role(StartingRole::ProjectLead), parent)
                .await?;
        }
        Actor::LeadOnSiblingOnly => {
            org.give(member, role(StartingRole::ProjectLead), sibling)
                .await?;
        }
        Actor::NoAccessOnTarget => {
            org.give_everywhere(member, role(StartingRole::Viewer))
                .await?;
            org.forbid(member, target).await?;
        }
        Actor::NoAccessUnderLead => {
            org.give_everywhere(member, role(StartingRole::ProjectLead))
                .await?;
            org.forbid(member, target).await?;
        }
        // An administrator who comes with a key: one that may only see, and one for the
        // sibling project (a key made by its owner, with the password asked again).
        Actor::KeySeeOnly | Actor::KeyForAnotherProject => {
            org.give_everywhere(member, role(StartingRole::Administrator))
                .await?;
            let (choice, projects) = if actor == Actor::KeySeeOnly {
                (KeyChoice::SeeOnly, Vec::new())
            } else {
                (KeyChoice::Everything, vec![sibling])
            };
            let made = rig
                .api()
                .make_access_key(
                    Rig::info(),
                    MakeAccessKey {
                        session_token: member.session.clone(),
                        password: PASSWORD.to_owned(),
                        name: actor.name(),
                        choice,
                        permissions: Vec::new(),
                        projects,
                        expires_at: None,
                    },
                )
                .await
                .map_err(|error| error.to_string())?
                .into_value();
            return Ok(Some(made.secret));
        }
    }
    Ok(None)
}

/// The starting roles of the organization by their key (`administrator`, …).
async fn starting_roles(org: &Org<'_>) -> Result<BTreeMap<String, RoleId>, Failure> {
    let list = org
        .rig
        .api()
        .roles(
            Rig::info(),
            RolesQuery {
                session_token: org.admin.session.clone(),
                place: None,
                cursor: None,
                limit: bb_core::paging::PAGE_SIZE_MAX,
                giveable: false,
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    Ok(list
        .roles
        .into_iter()
        .filter_map(|role| role.starting.map(|key| (key, role.id)))
        .collect())
}

/// What a route that is let through must hold for each actor, besides not being refused: only the
/// list of projects says anything about its content (M2-F27: *a key for A lists only A and below;
/// the trail stops at A*).
fn check_content(actor: Actor, route: &Operation, reply: &Reply) -> Result<(), String> {
    match route.id.as_str() {
        "listProjects" => {
            let names: std::collections::BTreeSet<&str> = reply.body["items"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|item| item["short_name"].as_str())
                .collect();
            let expected = top_of_the_tree(actor);
            if names == expected {
                Ok(())
            } else {
                Err(format!(
                    "lists {names:?}, the actor sees {expected:?} at the top"
                ))
            }
        }
        "getProject" => {
            // The way down to the target: the parent when the actor sees it, nothing when not.
            let trail: Vec<&str> = reply.body["trail"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|step| step["short_name"].as_str())
                .collect();
            let expected: Vec<&str> = everything_seen(actor)
                .into_iter()
                .filter(|name| *name == short_name(Project::Parent))
                .collect();
            if trail == expected {
                Ok(())
            } else {
                Err(format!("the trail is {trail:?}, expected {expected:?}"))
            }
        }
        _ => Ok(()),
    }
}

/// Runs every route as every actor; the record of what ran.
async fn run(rig: &Rig, actors: &[Actor]) -> Result<Record, Failure> {
    let fixture = Fixture::start(rig).await?;
    let routes = operations();
    // Besides the actors of the list, each route is run by the person who holds exactly the
    // permission it declares.
    let mut named = actors.to_vec();
    named.push(Actor::Exactly(None));
    let mut record = Record::new(&named);
    let mut browsers = BTreeMap::new();
    for actor in &named {
        browsers.insert(actor.key(), fixture.browser(*actor).await?);
    }
    for permission in Permission::ALL {
        let actor = Actor::Exactly(Some(*permission));
        browsers.insert(actor.key(), fixture.browser(actor).await?);
    }
    for route in routes {
        let example = EXAMPLES
            .iter()
            .find(|example| example.operation == route.id)
            .ok_or_else(|| {
                format!(
                    "{} {} ({}) has no example request in examples.rs",
                    route.method, route.path, route.id
                )
            })?;
        let mut route_actors = actors.to_vec();
        route_actors.push(Actor::exactly(route.needs, example));
        for actor in &route_actors {
            let browser = browsers
                .get_mut(&actor.key())
                .ok_or("no browser for the actor")?;
            let (method, address, body) = request(&fixture, example, *actor, browser).await?;
            if method != route.method.as_str() {
                return Err(format!(
                    "{}: the example is a {method}, the route a {}",
                    route.id, route.method
                )
                .into());
            }
            let reply = browser.send(method, &address, body).await?;
            let place = || {
                format!(
                    "{} {} as {}: {} {}",
                    route.method,
                    route.path,
                    actor.name(),
                    reply.status,
                    reply.code()
                )
            };
            assert!(!reply.status.is_server_error(), "{}", place());
            let expected = expectation(*actor, route, example);
            let got = seen(&reply);
            // A refusal that nothing expects is a failure, not an answer that was let in.
            let refusal = matches!(
                reply.status,
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
            ) && !KNOWN_REFUSALS.contains(&reply.code())
                && !example.answers(reply.code());
            assert!(!refusal, "{}: an unexpected refusal", place());
            let fine = got == expected
                || (expected == Expect::Allowed
                    && example.has(Quirk::NamesAMissingThing)
                    && got == Expect::NotFound);
            assert!(fine, "{}: expected {expected:?}, got {got:?}", place());
            if let (Expect::Allowed, Err(wrong)) = (expected, check_content(*actor, route, &reply))
            {
                panic!("{}: {wrong}", place());
            }
            if example.has(Quirk::EndsTheSession)
                && expected == Expect::Allowed
                && actor.signed_in()
            {
                browsers.insert(actor.key(), fixture.browser(*actor).await?);
            }
            record.ran(route, *actor, expected);
        }
    }
    Ok(record)
}

#[tokio::test]
async fn sec_01_every_route_answers_every_actor_as_the_permission_table_says() -> TestResult {
    each_database("sec_matrix_every_route_every_actor", async |db| {
        let rig = Rig::start_with(db, Tries::Unreached).await?;
        let record = run(&rig, &actors()).await?;
        record.matches_the_repository()?;
        Ok(())
    })
    .await
}

#[test]
fn every_route_of_the_description_has_a_row_and_every_row_a_route() {
    let described: std::collections::BTreeSet<&str> =
        operations().iter().map(|route| route.id.as_str()).collect();
    let rows: std::collections::BTreeSet<&str> =
        EXAMPLES.iter().map(|example| example.operation).collect();
    assert_eq!(
        described.difference(&rows).collect::<Vec<_>>(),
        Vec::<&&str>::new(),
        "routes without a row in examples.rs"
    );
    assert_eq!(
        rows.difference(&described).collect::<Vec<_>>(),
        Vec::<&&str>::new(),
        "rows without a route"
    );
}

#[test]
fn the_expectation_is_a_refusal_for_a_person_who_lacks_the_permission_and_hides_what_they_may_not_know()
-> Result<(), Failure> {
    let find = |operation: &str| {
        operations()
            .iter()
            .find(|route| route.id == operation)
            .zip(
                EXAMPLES
                    .iter()
                    .find(|example| example.operation == operation),
            )
            .map(|(route, example)| (route, *example))
            .ok_or("no such route")
    };
    let (rename, rename_example) = find("changeProject")?;
    let lacks = Actor::Role(StartingRole::Viewer);
    assert_eq!(
        expectation(lacks, rename, &rename_example),
        Expect::Forbidden
    );
    assert_eq!(
        expectation(Actor::LeadOnSiblingOnly, rename, &rename_example),
        Expect::NotFound
    );
    assert_eq!(
        expectation(Actor::Nobody, rename, &rename_example),
        Expect::NotSignedIn
    );
    // Under a *No access* a lead keeps deciding roles, so they know the project: refused, not
    // hidden; but what is inside the project is hidden from them.
    assert_eq!(
        expectation(Actor::NoAccessUnderLead, rename, &rename_example),
        Expect::Forbidden
    );
    let (read, read_example) = find("getProject")?;
    assert_eq!(
        expectation(Actor::NoAccessUnderLead, read, &read_example),
        Expect::NotFound
    );
    let (list, list_example) = find("listProjects")?;
    assert_eq!(
        expectation(Actor::NoRole, list, &list_example),
        Expect::Allowed,
        "M2-F30: the list filters what is seen"
    );
    Ok(())
}
