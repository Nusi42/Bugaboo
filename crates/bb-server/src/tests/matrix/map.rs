//! The relation map in the permission matrix (SEC-MATRIX, ISS-16, PRIV-13): for every starting
//! role, a person of the target project reads the map of an issue and the map of a search, and the
//! record has a line for each that says whether anything of a hidden issue was on it. The entry
//! point is `map`; its lines in `tools/checks/sec-matrix-record.txt` start with `map`.
//!
//! The scene: in ACME the start blocks an issue the person may see and one of a project nobody but
//! the administrator sees, which blocks one more issue of ACME. The map of the start must show
//! neither the hidden issue nor the one behind it, which only the hidden one joins to the start;
//! the map of everything the person sees must show no line to the hidden issue. Beside it, the
//! property tests `iss_16_no_chain_passes_through_a_hidden_issue` and
//! `iss_16_the_map_is_the_map_of_the_issues_the_reader_sees` (`bb-app`) try random graphs.

use bb_api_words::sec_matrix as words;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestDatabase};
use serde_json::{Value, json};

use super::record::matches_the_repository_for;
use crate::sign_in_tests::http::rig_and_app;
use crate::sign_in_tests::issues_site::{ISSUES, PROJECTS, Site, id_of, key_of, ok};
use crate::sign_in_tests::relations_http::kinds;

/// What one starting role was shown on one kind of map.
pub(super) struct Mapped {
    role: StartingRole,
    kind: &'static str,
    /// Whether the hidden issue was named anywhere in the answer.
    hidden: bool,
    /// Whether the issue behind it was, on the map of an issue.
    behind: Option<bool>,
}

impl Mapped {
    /// The line of the record.
    pub(super) fn line(&self) -> String {
        let word = |seen: bool| if seen { words::PRESENT } else { words::ABSENT };
        let behind = self
            .behind
            .map(|seen| format!(" behind={}", word(seen)))
            .unwrap_or_default();
        format!(
            "{}{} {} : hidden={}{behind}",
            words::MAP_LINE,
            self.kind,
            words::role_actor(self.role.code()),
            word(self.hidden),
        )
    }
}

/// Whether the answer names the issue by its key or its ID, anywhere.
fn names(answer: &Value, issue: &(String, String)) -> bool {
    let text = answer.to_string();
    text.contains(&issue.0) || text.contains(&issue.1)
}

/// Every starting role reads both maps; the lines of what each was shown.
///
/// # Errors
/// The server answers something the scene does not expect.
pub(super) async fn run(db: &TestDatabase) -> Result<Vec<Mapped>, Failure> {
    let (rig, app) = rig_and_app(db).await?;
    let (site, mut ada) = Site::start(&rig, &app).await?;
    let (blocks, _) = kinds(&mut ada).await?;
    let secret = ok(&ada
        .send(
            "POST",
            PROJECTS,
            Some(json!({"short_name": "SECRET", "name": "Secret"})),
        )
        .await?)?["id"]
        .as_str()
        .ok_or("no project ID")?
        .to_owned();
    let mut made = Vec::new();
    for title in ["Start", "Seen", "Behind"] {
        let reply = site.create(&mut ada, title).await?;
        made.push((key_of(&reply)?, id_of(&reply)?));
    }
    let reply = ada
        .send(
            "POST",
            &format!("{PROJECTS}/{secret}/issues"),
            Some(site.new_issue("Hidden")),
        )
        .await?;
    let hidden = (key_of(&reply)?, id_of(&reply)?);
    for (from, to) in [
        (&made[0], &made[1]),
        (&made[0], &hidden),
        (&hidden, &made[2]),
    ] {
        ok(&ada
            .send(
                "POST",
                &format!("{ISSUES}/{}/relations", from.0),
                Some(json!({"other": to.0, "relation_kind": blocks, "direction": "forward"})),
            )
            .await?)?;
    }
    let mut everyone = Vec::new();
    for role in StartingRole::ALL.iter().copied() {
        let (mut person, _) = site
            .person_in_acme(role, &format!("maps-{}", role.code()))
            .await?;
        // A role that does not see the start is told it is not there, which shows nothing.
        let of_issue = person
            .send("GET", &format!("{ISSUES}/{}/map", made[0].0), None)
            .await?;
        everyone.push(Mapped {
            role,
            kind: words::MAP_OF_ISSUE,
            hidden: names(&of_issue.body, &hidden),
            behind: Some(of_issue.status.is_success() && names(&of_issue.body, &made[2])),
        });
        let of_search = person
            .send("POST", "/api/v1/map", Some(json!({"search_text": ""})))
            .await?;
        everyone.push(Mapped {
            role,
            kind: words::MAP_OF_SEARCH,
            hidden: names(&of_search.body, &hidden),
            behind: None,
        });
    }
    Ok(everyone)
}

#[tokio::test]
async fn sec_01_the_relation_map_never_shows_or_walks_through_a_hidden_issue()
-> bb_testing::TestResult {
    bb_testing::each_database("sec_matrix_map", async |db| {
        let mapped = run(db).await?;
        for each in &mapped {
            assert!(!each.hidden, "{}", each.line());
            assert_ne!(each.behind, Some(true), "{}", each.line());
        }
        let lines: Vec<String> = mapped.iter().map(Mapped::line).collect();
        matches_the_repository_for(words::MAP_LINE, &lines)
    })
    .await
}
