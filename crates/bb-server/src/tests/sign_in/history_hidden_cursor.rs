//! The cursor of the history never names a row the reader may not see (M3-F68, SEC-06): a long run
//! of changes of a field the reader may not see ends a page with a cursor that holds no row, the
//! pages that follow find every change the reader may see once, and the end of the history has no
//! cursor, as for a reader without hidden rows. Both databases.

use std::collections::BTreeMap;

use bb_app::access::Permission;
use bb_app::issues::{ChangeIssue, IssueQuery};
use bb_core::access::FieldLevel;
use bb_core::paging::LOOKED_AT_MAX;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::fields_http::{create_field, reader_of};
use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, id_in, key_in, ok};
use super::rig::Rig;

/// Changes the salary band, or the title, of an issue, whatever version it has now.
async fn change(ada: &mut Browser, issue: &str, change: Value) -> TestResult {
    let path = format!("{ISSUES}/{issue}");
    let version = ok(&ada.send("GET", &path, None).await?)?["issue"]["version"].clone();
    let mut body = change;
    body["version"] = version;
    ok(&ada.send("PATCH", &path, Some(body)).await?)?;
    Ok(())
}

/// Changes the salary band `count` times through the use cases (the requests of the API are
/// limited for each minute, and this is the long run of changes the test needs).
async fn salary_changes(
    rig: &Rig,
    session: &str,
    issue: &str,
    salary_id: &str,
    count: usize,
) -> TestResult {
    for number in 0..count {
        let version = rig
            .api()
            .issue(
                Rig::info(),
                IssueQuery {
                    session_token: session.to_owned(),
                    issue: issue.to_owned(),
                },
            )
            .await?
            .into_value()
            .issue
            .version;
        rig.api()
            .change_issue(
                Rig::info(),
                ChangeIssue {
                    session_token: session.to_owned(),
                    issue: issue.to_owned(),
                    version,
                    issue_type: None,
                    title: None,
                    status: None,
                    priority: None,
                    assigned_to: None,
                    fields: BTreeMap::from([(
                        salary_id.to_owned(),
                        json!(format!("band {number}")),
                    )]),
                },
            )
            .await?;
    }
    Ok(())
}

/// Every page of the changes of an issue from the first: the entries of each page and its cursor.
async fn walked(
    reader: &mut Browser,
    issue: &str,
    limit: u32,
) -> Result<Vec<(Vec<Value>, Option<String>)>, Failure> {
    let mut pages = Vec::new();
    let mut cursor: Option<String> = None;
    while pages.len() < 40 {
        let path = match &cursor {
            Some(cursor) => {
                format!("{ISSUES}/{issue}/history?tab=changes&limit={limit}&cursor={cursor}")
            }
            None => format!("{ISSUES}/{issue}/history?tab=changes&limit={limit}"),
        };
        let reply = reader.send("GET", &path, None).await?;
        let body = ok(&reply)?;
        let entries = body["items"].as_array().cloned().unwrap_or_default();
        cursor = body["next_cursor"].as_str().map(str::to_owned);
        pages.push((entries, cursor.clone()));
        if cursor.is_none() {
            break;
        }
    }
    Ok(pages)
}

#[tokio::test]
async fn iss_06_a_long_run_of_hidden_changes_leaves_a_cursor_that_names_no_row() -> TestResult {
    each_database("iss_06_hidden_cursor", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let salary = create_field(
            &mut ada,
            json!({"name": "Salary band", "kind": "text", "applies_to": "issues"}),
        )
        .await?;
        let salary_id = id_in(&salary)?;
        let issue = key_in(&site.create(&mut ada, "An issue").await?.body)?;
        // The reader sees titles and not the salary band.
        let mut reader = reader_of(
            &site,
            &rig,
            "hist@example.test",
            &[Permission::SeeProjects, Permission::SeeIssues],
            &[
                ("title", None, FieldLevel::CanSee),
                (&salary_id, None, FieldLevel::Hidden),
            ],
        )
        .await?;
        change(&mut ada, &issue, json!({"title": "The first title"})).await?;
        // More changes the reader may not see than one request looks at, then one they may.
        salary_changes(
            &rig,
            &site.org.admin.session,
            &issue,
            &salary_id,
            LOOKED_AT_MAX + 5,
        )
        .await?;
        change(&mut ada, &issue, json!({"title": "The last title"})).await?;

        let pages = walked(&mut reader, &issue, 50).await?;
        // The first page ends after the rows looked at, with a cursor that says there is more
        // and names no row (the sealed text does not tell, the pages that follow do).
        assert!(pages.len() >= 2, "{} pages", pages.len());
        assert!(
            pages[0].1.is_some(),
            "a page that looked at its fill has a cursor"
        );
        let titles: Vec<&Value> = pages
            .iter()
            .flat_map(|(entries, _)| entries)
            .filter(|entry| entry["change"]["field"] == json!("title"))
            .collect();
        let after: Vec<&str> = titles
            .iter()
            .filter_map(|entry| entry["change"]["after"]["text"].as_str())
            .collect();
        assert_eq!(after.first(), Some(&"The last title"), "{after:?}");
        assert_eq!(
            after
                .iter()
                .filter(|text| **text == "The first title")
                .count(),
            1
        );
        // Nothing of the salary band is in any page, and the end of the history has no cursor.
        let text = json!(pages.iter().map(|(entries, _)| entries).collect::<Vec<_>>()).to_string();
        for hidden in [salary_id.as_str(), "Salary band", "band 7"] {
            assert!(!text.contains(hidden), "{hidden} is in the history");
        }
        assert_eq!(pages.last().map(|(_, cursor)| cursor.clone()), Some(None));
        // A reader who sees every field reads the same changes of the title, and has the same
        // kind of end.
        let everything = walked(&mut ada, &issue, 50).await?;
        let all_titles = everything
            .iter()
            .flat_map(|(entries, _)| entries)
            .filter(|entry| entry["change"]["field"] == json!("title"))
            .count();
        assert_eq!(all_titles, titles.len());
        assert_eq!(
            everything.last().map(|(_, cursor)| cursor.clone()),
            Some(None)
        );
        Ok(())
    })
    .await
}
