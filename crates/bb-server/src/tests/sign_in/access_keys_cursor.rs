//! The cursor of the list of projects for a key holds nothing a person may not see (PERF-02,
//! AUTH-07, ADR-0025 decision 12): a page that stops after it looked at many projects the person
//! does not know about must not carry the name of one of them in its cursor, which is readable.

use bb_app::access::Permission;
use bb_app::auth::ProjectChoicesQuery;
use bb_testing::each_database;

use super::policy_rig::{Member, Org, Run};
use super::rig::{Outcome, Rig};

/// How many projects there are: more than a page of the list looks at (500), all of them
/// unknown to the people of the test, except the one the reader is given.
const PROJECTS: usize = 520;

/// The pages of the list of `who`: their items and the cursor each one hands out.
async fn pages(rig: &Rig, who: &Member, limit: u32) -> Run<Vec<(Vec<String>, Option<String>)>> {
    let mut found = Vec::new();
    let mut cursor = None;
    loop {
        let page = rig
            .api()
            .project_choices(
                Rig::info(),
                ProjectChoicesQuery {
                    session_token: who.session.clone(),
                    cursor: cursor.clone(),
                    limit,
                },
            )
            .await?
            .into_value();
        let items = page
            .items
            .iter()
            .map(|project| project.short_name.clone())
            .collect();
        cursor = page.next_cursor;
        found.push((items, cursor.clone()));
        if cursor.is_none() {
            return Ok(found);
        }
        assert!(found.len() < 20, "the pages do not end");
    }
}

/// Every cursor of the pages is made only of what the person has been shown: the short name of
/// a project of a page they got (or none) and a whole number of stretches that were looked at.
fn assert_nothing_hidden(found: &[(Vec<String>, Option<String>)]) {
    let mut shown: Vec<&String> = Vec::new();
    for (items, cursor) in found {
        shown.extend(items);
        let Some(cursor) = cursor else { continue };
        let (name, skip) = cursor.split_once('+').unwrap_or((cursor, "0"));
        assert!(
            name.is_empty() || shown.iter().any(|seen| seen.as_str() == name),
            "the cursor {cursor:?} names a project the person was not shown"
        );
        assert!(
            skip.parse::<u64>().is_ok_and(|skip| skip % 500 == 0),
            "the cursor {cursor:?} counts more than whole stretches"
        );
    }
}

#[tokio::test]
async fn auth_07_the_cursor_of_the_projects_for_a_key_holds_no_name_of_a_project_hidden_from_the_person()
-> Outcome {
    each_database("auth_07_choices_cursor_holds_nothing_hidden", async |db| {
        let rig = Rig::start(db, 50).await?;
        let org = Org::start(&rig).await?;
        for number in 0..PROJECTS {
            org.project(None, &format!("H{number:03}")).await?;
        }
        // The reader holds a role in the project that is last in the order, so the first page
        // looks at 500 projects she does not know about and stops there.
        let last = org.project(None, "ZLAST").await?;
        let role = org.role("Reader", &[Permission::SeeProjects]).await?;
        let reader = org.person().await?;
        org.give(&reader, role.id, last.id).await?;
        let found = pages(&rig, &reader, 5).await?;
        let seen: Vec<&String> = found.iter().flat_map(|(items, _)| items).collect();
        assert_eq!(seen, ["ZLAST"], "she finds the one project she knows, once");
        assert_eq!(found[0].1.as_deref(), Some("+500"));
        assert_nothing_hidden(&found);
        // Somebody who knows no project is walked through the same pages and learns no name.
        let nobody = org.person().await?;
        let found = pages(&rig, &nobody, 5).await?;
        assert!(found.iter().all(|(items, _)| items.is_empty()));
        assert_nothing_hidden(&found);
        // Somebody who knows the first two projects, a page of one at a time: a full page goes on
        // after its last project, and a hidden stretch is passed in a whole step.
        let early = org.person().await?;
        for name in ["H000", "H001"] {
            let project = org.project(None, &format!("{name}X")).await?;
            org.give(&early, role.id, project.id).await?;
        }
        let found = pages(&rig, &early, 1).await?;
        let seen: Vec<&String> = found.iter().flat_map(|(items, _)| items).collect();
        assert_eq!(seen, ["H000X", "H001X"]);
        assert_eq!(found[0].1.as_deref(), Some("H000X"));
        assert_nothing_hidden(&found);
        Ok(())
    })
    .await
}
