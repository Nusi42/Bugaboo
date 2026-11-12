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

#[tokio::test]
async fn perf_05_a_read_of_one_persons_work_times_runs_as_many_statements_with_twenty_projects_as_with_two()
-> TestResult {
    each_database("perf_05_work_times_projects", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let roles = site.org.starting_roles().await?;
        let member = roles[StartingRole::TeamMember.code()];
        let (mut reader, reader_member) = site.person_holding(member, "reader", &site.acme).await?;
        let (_target, target_member) = site.person_holding(member, "target", &site.acme).await?;
        let path = format!(
            "/api/v1/people/{}/work-times?from=2027-01-01&to=2027-12-31",
            target_member.id
        );
        let mut counter = Counter::on(&rig.database);
        // ACME and one more project, both shared by the two people.
        for number in 0..1 {
            let other = project(&mut ada, &format!("PA{number}")).await?;
            let id = other.parse().map_err(|_| "no project ID")?;
            site.org.give(&reader_member, member, id).await?;
            site.org.give(&target_member, member, id).await?;
        }
        ok(&reader.send("GET", &path, None).await?)?;
        counter.take();
        let few = reader.send("GET", &path, None).await?;
        assert_eq!(ok(&few)?["person"]["shown"], "name");
        let (on_two, two_log) = counter.take_logged();
        for number in 1..19 {
            let other = project(&mut ada, &format!("PA{number}")).await?;
            let id = other.parse().map_err(|_| "no project ID")?;
            site.org.give(&reader_member, member, id).await?;
            site.org.give(&target_member, member, id).await?;
        }
        ok(&reader.send("GET", &path, None).await?)?;
        counter.take();
        let many = reader.send("GET", &path, None).await?;
        assert_eq!(ok(&many)?["person"]["shown"], "name");
        let (on_twenty, twenty_log) = counter.take_logged();
        assert_same_statements(&two_log, &twenty_log);
        assert_as_many_statements(on_two, on_twenty, 60, "more projects in common");
        Ok(())
    })
    .await
}
