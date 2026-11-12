//! No N+1 queries in the shares of a project (PERF-05, PERF-N1): the people of a project with their
//! shares, for a reader who decides who has which role there and so also reads the sum and the
//! other projects of each person, runs as many statements for forty people as for five, on SQLite
//! and PostgreSQL.

use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::http::{Browser, rig_and_app};
use super::issues_site::{PROJECTS, Site, ok};
use super::perf_queries::{Counter, assert_as_many_statements, assert_same_statements};
use super::work_times_http::project;

/// The most statements the list runs, however many people it has: the session, the rights of the
/// reader, the page of people, their rights in the project and in the other projects, the shares,
/// and the names.
const AT_MOST: usize = 60;

/// Adds the people `from..to` to ACME, each with a share there and one in the other project.
async fn add_people(
    site: &Site<'_>,
    ada: &mut Browser,
    other: &str,
    from: usize,
    to: usize,
) -> Result<(), Failure> {
    for number in from..to {
        let (_browser, person) = site
            .person_in_acme(StartingRole::TeamMember, &format!("p{number}"))
            .await?;
        for (project, percent) in [(&site.acme, 60), (&other.to_owned(), 50)] {
            let set = ada
                .send(
                    "PUT",
                    &format!("{PROJECTS}/{project}/shares/{person}"),
                    Some(json!({"percent": percent})),
                )
                .await?;
            ok(&set)?;
        }
    }
    Ok(())
}

#[tokio::test]
async fn perf_05_forty_people_with_their_shares_run_as_many_statements_as_five() -> TestResult {
    each_database("perf_05_project_shares", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let other = project(&mut ada, "OTHER").await?;
        let (mut bea, _) = site
            .person_in_acme(StartingRole::ProjectLead, "bea")
            .await?;
        let path = format!("{PROJECTS}/{}/shares", site.acme);
        let mut counter = Counter::on(&rig.database);
        add_people(&site, &mut ada, &other, 0, 3).await?;
        ok(&bea.send("GET", &path, None).await?)?;
        counter.take();
        let few = bea.send("GET", &path, None).await?;
        // The three, Bea and the administrator.
        assert_eq!(ok(&few)?["items"].as_array().map(Vec::len), Some(5));
        let (on_five, five_log) = counter.take_logged();
        add_people(&site, &mut ada, &other, 3, 38).await?;
        ok(&bea.send("GET", &path, None).await?)?;
        counter.take();
        let many = bea.send("GET", &path, None).await?;
        assert_eq!(ok(&many)?["items"].as_array().map(Vec::len), Some(40));
        let (on_forty, forty_log) = counter.take_logged();
        assert_same_statements(&five_log, &forty_log);
        assert_as_many_statements(on_five, on_forty, AT_MOST, "more people in the project");
        Ok(())
    })
    .await
}
