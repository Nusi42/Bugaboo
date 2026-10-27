//! The acceptance tests of the saved searches of the sample (M5 WP-10, QRY-06): people of every
//! role saved a search, the shared ones reach the people they are shared with, the one that reads
//! an own field that Contributors may not read never reaches a Contributor, and a
//! project shows one as its list. They make the real thing on SQLite and PostgreSQL and read it
//! back through the use cases, as each person would.

use std::future::Future;

use bb_app::auth::{SignIn, sign_in};
use bb_app::saved_searches::{ProjectListQuery, SearchesQuery, project_list, saved_searches};
use bb_core::saved_search::SearchSet;
use bb_testing::{Failure, TestResult, each_database};
use jiff::Timestamp;
use serde_json::Value;

use crate::{
    DEFAULT_SEED, Everything, Later, LeftBehind, Options, SampleError, Step, Target, World, create,
};

const START: i64 = bb_dev_words::accounts::CLOCK_START;
/// Few enough to be quick, enough for every kind of work to be there.
const ISSUES: u32 = 150;
/// How many a page holds when a person reads their searches: more than the sample makes.
const PAGE: u32 = 100;

/// What one person sees of the saved searches.
struct Seen {
    role: String,
    mine: Vec<String>,
    shared: Vec<(String, String)>,
}

/// The sample with everything, and then each person reading their searches.
struct Reading {
    everything: Everything,
    seen: Vec<Seen>,
    web_list: Option<String>,
}

impl Later for Reading {
    fn history_seconds(&self) -> i64 {
        self.everything.history_seconds()
    }

    fn left_behind(&self) -> LeftBehind {
        self.everything.left_behind()
    }

    fn after_people(&mut self, world: &World<'_>) -> impl Future<Output = Result<(), SampleError>> {
        async move {
            self.everything.after_people(world).await?;
            let step =
                |error: &dyn std::fmt::Display| SampleError::step(Step::SavedSearches, &error);
            for made in world.people.iter().filter(|made| !made.person.code_app) {
                let ctx = world.services.ctx(world.request());
                let started = sign_in(
                    &ctx,
                    SignIn {
                        email: made.person.email.clone(),
                        password: made.password.clone(),
                        previous_session: None,
                    },
                )
                .await
                .map_err(|error| SampleError::step(Step::SigningIn, &error))?;
                let mut lists = Vec::new();
                for scope in [SearchSet::Mine, SearchSet::Shared] {
                    let page = saved_searches(
                        &ctx,
                        SearchesQuery {
                            session_token: started.session_token.clone(),
                            scope,
                            cursor: None,
                            limit: PAGE,
                        },
                    )
                    .await
                    .map_err(|error| step(&error))?
                    .masked_value()
                    .map_err(|error| step(&error))?;
                    lists.push(page);
                }
                let names = |page: &Value| -> Vec<(String, String)> {
                    page["items"].as_array().map_or(Vec::new(), |items| {
                        items
                            .iter()
                            .map(|item| {
                                (
                                    item["name"].as_str().unwrap_or_default().to_owned(),
                                    item["text"].as_str().unwrap_or_default().to_owned(),
                                )
                            })
                            .collect()
                    })
                };
                if self.web_list.is_none() && made.person.role.code() == "team-member" {
                    let list = project_list(
                        &ctx,
                        ProjectListQuery {
                            session_token: started.session_token.clone(),
                            project: "WEB".to_owned(),
                        },
                    )
                    .await;
                    // A Team member of a project that is not WEB does not see its list.
                    if let Ok(list) = list {
                        let list = list.masked_value().map_err(|error| step(&error))?;
                        self.web_list = list["search"]["name"].as_str().map(str::to_owned);
                    }
                }
                self.seen.push(Seen {
                    role: made.person.role.code().to_owned(),
                    mine: names(&lists[0]).into_iter().map(|(name, _)| name).collect(),
                    shared: names(&lists[1]),
                });
            }
            Ok(())
        }
    }
}

async fn read_back(target: &Target, folder: &std::path::Path) -> Result<Reading, Failure> {
    let options = Options {
        seed: DEFAULT_SEED,
        start: Timestamp::from_second(START)?,
    };
    let mut reading = Reading {
        everything: Everything::new(ISSUES)?,
        seen: Vec::new(),
        web_list: None,
    };
    let sample = create(&options, folder, target, &mut reading).await?;
    sample.close().await;
    Ok(reading)
}

#[tokio::test]
async fn qry_06_every_role_saved_a_search_and_the_shared_ones_reach_only_who_reads_them()
-> TestResult {
    each_database("qry_06_the_sample_saved_searches", async |db| {
        let folder = tempfile::tempdir()?;
        let reading = read_back(&Target::Address(db.url().to_owned()), folder.path()).await?;
        let plan = bb_dev_words::sample::Layout::bundled()?.issues.work;
        // Every search is in the list of the first person of its role who made one.
        for search in &plan.saved_searches {
            assert!(
                reading
                    .seen
                    .iter()
                    .any(|seen| seen.role == search.maker && seen.mine.contains(&search.name)),
                "{} of the {} is not in their own searches",
                search.name,
                search.maker
            );
        }
        // The roles that have searches of their own: every role of the plan.
        for role in [
            "administrator",
            "team-member",
            "contributor",
            "viewer",
            "guest",
        ] {
            assert!(
                reading
                    .seen
                    .iter()
                    .any(|seen| seen.role == role && !seen.mine.is_empty()),
                "nobody of the role {role} has a search of their own"
            );
        }
        // A search that reads a field Contributors may not read never reaches a Contributor; every other shared search that reaches them checks cleanly (they are listed only
        // when it does), and Team members of the groups and projects it is shared with get it.
        let hidden = plan
            .hidden_from_contributors
            .first()
            .ok_or("no hidden field")?;
        let hidden_word = hidden.to_lowercase();
        for seen in &reading.seen {
            if seen.role == "contributor" {
                for (name, text) in &seen.shared {
                    assert!(
                        !text.to_lowercase().contains(&hidden_word),
                        "{name} reads {hidden} and reached a Contributor"
                    );
                }
            }
        }
        // The ones shared with a project do reach a Contributor who sees it.
        let for_the_project = plan
            .saved_searches
            .iter()
            .find(|search| search.project_list.is_some())
            .ok_or("no search is shared with a project")?;
        assert!(
            reading.seen.iter().any(|seen| seen.role == "contributor"
                && seen
                    .shared
                    .iter()
                    .any(|(name, _)| name == &for_the_project.name)),
            "no Contributor got {}",
            for_the_project.name
        );
        let questions = plan
            .saved_searches
            .iter()
            .find(|search| search.text.to_lowercase().contains(&hidden_word))
            .ok_or("no search reads the hidden field")?;
        assert!(
            reading.seen.iter().any(|seen| seen.role == "team-member"
                && seen.shared.iter().any(|(name, _)| name == &questions.name)),
            "no Team member got {}",
            questions.name
        );
        // A project shows one of the searches as its list.
        let listed = plan
            .saved_searches
            .iter()
            .find(|search| search.project_list.as_deref() == Some("WEB"))
            .ok_or("no search is the list of WEB")?;
        assert_eq!(reading.web_list.as_deref(), Some(listed.name.as_str()));
        Ok(())
    })
    .await
}
