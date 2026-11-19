//! The switches of an agent in the permission matrix (SEC-MATRIX, AI-31, AI-38, ADR-0029 amendment
//! M9): the member `switches` of `changeAgent` has a rule of its own beside the other members of
//! the route. An administrator may stop any agent, but only the agent's person widens what it does,
//! so the route's own row, whose example stops an agent nobody has, does not show it. This test runs
//! the member for every starting role, the agent's person, a key of that person and the agent's own
//! key, against one agent of a team member, on both databases. Its lines in
//! `tools/checks/sec-matrix-record.txt` start with `member`; the route matrix keeps them as they are.

use bb_api::openapi::{Operation, operations};
use bb_api_words::sec_matrix as words;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::record::{matches_the_repository_for, word};
use super::routes::{Expect, seen};
use crate::sign_in_tests::ai_mcp_rig::an_agent_with_a_key;
use crate::sign_in_tests::http::{Browser, Reply, called, rig_and_app};
use crate::sign_in_tests::issues_site::{Site, ok};
use crate::sign_in_tests::rig::PASSWORD;

/// The route whose member this is.
const ROUTE: &str = "changeAgent";
/// The member of the body with a rule of its own.
const MEMBER: &str = "switches";

/// The body that turns both switches on, on the version the agent was made with.
fn both_on() -> Value {
    json!({"version": 1, MEMBER: {"may_hand_over": true, "may_undo": true}})
}

/// Who asks and how: with a session, with a key, or not signed in.
enum Way {
    Session(Browser),
    Key(String),
    Nobody,
}

/// One actor of the line: its name in the record, how it asks and what it must be answered.
struct Asker {
    name: String,
    way: Way,
    expected: Expect,
}

impl Asker {
    async fn ask(&mut self, script: &mut Browser, path: &str) -> Result<Reply, Failure> {
        match &mut self.way {
            Way::Session(browser) => browser.send("PATCH", path, Some(both_on())).await,
            Way::Key(key) => script.script("PATCH", path, Some(both_on()), key).await,
            Way::Nobody => script.send("PATCH", path, Some(both_on())).await,
        }
    }
}

/// The actors: nobody, a person with each starting role who is not the agent's person (the
/// administrator among them), then a key of the agent's person, the agent's own key and, last, the
/// agent's person, whose change counts the version of the agent up.
async fn askers(site: &Site<'_>) -> Result<(Vec<Asker>, String), Failure> {
    let mut askers = vec![Asker {
        name: words::NOBODY.to_owned(),
        way: Way::Nobody,
        expected: Expect::NotSignedIn,
    }];
    for role in StartingRole::ALL {
        let (person, _) = site
            .person(*role, &format!("matrix-{}", role.code()))
            .await?;
        askers.push(Asker {
            name: words::role_actor(role.code()),
            way: Way::Session(person),
            expected: Expect::NotFound,
        });
    }
    let (mut person, _) = site
        .person(StartingRole::TeamMember, "matrix-agents-person")
        .await?;
    let (agent, agents_key) = an_agent_with_a_key(&mut person, "Agent of the matrix", None).await?;
    let made = person
        .send(
            "POST",
            "/api/v1/access-keys",
            Some(json!({"password": PASSWORD, "name": "Matrix", "choice": "see-and-change"})),
        )
        .await?;
    let persons_key = ok(&made)?["secret"].as_str().ok_or("no secret")?.to_owned();
    askers.extend([
        Asker {
            name: words::KEY_OF_THE_AGENTS_PERSON.to_owned(),
            way: Way::Key(persons_key),
            expected: Expect::NotForKeys,
        },
        Asker {
            name: words::AGENT_WITHOUT_WORK.to_owned(),
            way: Way::Key(agents_key),
            expected: Expect::NotForKeys,
        },
        Asker {
            name: words::THE_AGENTS_PERSON.to_owned(),
            way: Way::Session(person),
            expected: Expect::Allowed,
        },
    ]);
    Ok((askers, agent))
}

/// The line of the member: every actor with what the server answered.
async fn line(db: &bb_testing::TestDatabase) -> Result<String, Failure> {
    let (rig, app) = rig_and_app(db).await?;
    rig.turn_ai_apps_and_agents_on().await?;
    let (site, _ada) = Site::start(&rig, &app).await?;
    let route: &Operation = operations()
        .iter()
        .find(|route| route.id == ROUTE)
        .ok_or("the route is not in the API description")?;
    let (mut askers, agent) = askers(&site).await?;
    let path = route.path.replace("{agent}", &agent);
    let mut script = Browser::new(&app, &called());
    let mut answers = Vec::new();
    for asker in &mut askers {
        let reply = asker.ask(&mut script, &path).await?;
        let got = seen(&reply);
        assert_eq!(
            got, asker.expected,
            "{ROUTE} {MEMBER} as {}: {} {}",
            asker.name, reply.status, reply.body
        );
        if got == Expect::Allowed {
            let changed = ok(&reply)?;
            assert_eq!(changed["may_hand_over"], json!(true), "{changed}");
            assert_eq!(changed["may_undo"], json!(true), "{changed}");
        }
        answers.push(format!("{}={}", asker.name, word(got)));
    }
    Ok(format!(
        "{}{ROUTE} {MEMBER} {} {} {} {} : {}",
        words::MEMBER_LINE,
        words::ENTRY_API,
        route.method,
        route.path,
        route.needs.key(),
        answers.join(" ")
    ))
}

#[tokio::test]
async fn sec_01_only_the_person_of_an_agent_changes_its_switches_whatever_the_role() -> TestResult {
    each_database("sec_matrix_agent_switches", async |db| {
        let found = line(db).await?;
        matches_the_repository_for(words::MEMBER_LINE, &[found])
    })
    .await
}
