//! The filter of a group of a search (QRY-04, M5 WP-08): every group says, as search text, which
//! issues it holds, and that text run as a search finds exactly the group's issues. A group the
//! person may not name has no filter, and a hidden project or person is never in the groups.

use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{PROJECTS, Site, id_in, ok};

pub(super) const GROUPS: &str = "/api/v1/search/issues/groups";
const COUNT: &str = "/api/v1/search/issues/count";
const ISSUES: &str = "/api/v1/search/issues";

pub(super) fn encoded(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                char::from(byte).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

/// The IDs of the issues a search finds, in any order.
async fn ids_of(who: &mut Browser, text: &str) -> Result<Vec<String>, Failure> {
    let found = who
        .send(
            "GET",
            &format!("{ISSUES}?q={}&fields=id&limit=100", encoded(text)),
            None,
        )
        .await?;
    let mut ids: Vec<String> = ok(&found)?["items"]
        .as_array()
        .ok_or("no items")?
        .iter()
        .filter_map(|item| item["id"].as_str().map(str::to_owned))
        .collect();
    ids.sort();
    Ok(ids)
}

/// Every group of the field `by` has a filter, and the filter run as a search (with `base` before
/// it) finds as many issues as the group says, the same ones that the count says.
pub(super) async fn every_group_selects_its_issues(
    who: &mut Browser,
    by: &str,
    base: &str,
) -> Result<usize, Failure> {
    let groups = who
        .send(
            "GET",
            &format!("{GROUPS}?by={by}&q={}", encoded(base)),
            None,
        )
        .await?;
    let groups = ok(&groups)?["groups"]
        .as_array()
        .ok_or("no groups")?
        .clone();
    let mut seen = 0_u64;
    for group in &groups {
        let filter = group["filter"]
            .as_str()
            .ok_or_else(|| format!("the group {group} of {by} has no filter"))?;
        let text = if base.is_empty() {
            filter.to_owned()
        } else {
            format!("{base} and {filter}")
        };
        let count = who
            .send("GET", &format!("{COUNT}?q={}", encoded(&text)), None)
            .await?;
        let issues = group["issues"].as_u64().ok_or("no count")?;
        assert_eq!(ok(&count)?["issues"], issues, "{by}: {text}");
        assert_eq!(
            ids_of(who, &text).await?.len() as u64,
            issues,
            "{by}: {text}"
        );
        seen += issues;
    }
    assert!(seen > 0, "{by} has no issues in its groups");
    Ok(groups.len())
}

pub(super) async fn create_in(who: &mut Browser, project: &str, body: Value) -> TestResult {
    let reply = who
        .send("POST", &format!("{PROJECTS}/{project}/issues"), Some(body))
        .await?;
    ok(&reply)?;
    Ok(())
}

#[tokio::test]
async fn qry_04_the_filter_of_every_group_selects_exactly_its_issues() -> TestResult {
    each_database("search_group_filters", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let beta = id_in(
            &ada.send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "BETA", "name": "Beta"})),
            )
            .await?
            .body,
        )?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        for (project, title, priority) in [
            (site.acme.as_str(), "A low one", "low"),
            (site.acme.as_str(), "A high one", "high"),
            (site.acme.as_str(), "A critical one", "critical"),
            (beta.as_str(), "Another high one", "high"),
        ] {
            let mut body = site.new_issue(title);
            body["priority"] = json!(priority);
            create_in(&mut bob, project, body).await?;
        }
        for by in [
            "priority",
            "statusGroup",
            "status",
            "type",
            "project",
            "reportedBy",
            "assignedTo",
        ] {
            every_group_selects_its_issues(&mut ada, by, "").await?;
        }
        // Inside a search the filter joins with `and`, and the groups still add up.
        every_group_selects_its_issues(&mut ada, "priority", "type is Task").await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_05_a_group_of_a_project_or_a_person_the_reader_may_not_see_is_not_there() -> TestResult
{
    each_database("search_group_hidden", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let secret = id_in(
            &ada.send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "SECRET", "name": "Secret"})),
            )
            .await?
            .body,
        )?;
        create_in(&mut ada, &secret, site.new_issue("Hidden work")).await?;
        create_in(&mut ada, &site.acme, site.new_issue("Open work")).await?;
        // A person who is a member of ACME only sees ACME: no group of theirs names SECRET.
        let (mut bob, _) = site.person_in_acme(StartingRole::TeamMember, "bob").await?;
        for by in ["project", "reportedBy"] {
            let groups = bob.send("GET", &format!("{GROUPS}?by={by}"), None).await?;
            let body = ok(&groups)?.to_string();
            assert!(
                !body.contains("SECRET") && !body.contains("Hidden"),
                "{by}: {body}"
            );
        }
        every_group_selects_its_issues(&mut bob, "project", "").await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_05_a_person_shown_only_by_role_is_never_named_in_the_groups() -> TestResult {
    each_database("search_group_role_only", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Builder")
            .await?;
        create_in(&mut bob, &site.acme, site.new_issue("By Bob")).await?;
        let (mut cora, _) = site
            .person_called(StartingRole::TeamMember, "cora", "Cora Hidden")
            .await?;
        // The team members see people who reported issues by role only.
        site.show_people(
            &mut ada,
            StartingRole::TeamMember,
            "reported-by",
            "role-only",
        )
        .await?;
        let groups = cora
            .send("GET", &format!("{GROUPS}?by=reportedBy"), None)
            .await?;
        // A kind whose names are not shown cannot be searched (ADR-0022): the groups by it are
        // refused like a field that does not exist, so the person is never named.
        assert_eq!(groups.status, axum::http::StatusCode::BAD_REQUEST);
        assert!(
            !groups.body.to_string().contains("Bob Builder"),
            "{}",
            groups.body
        );
        // Where names are shown, the same group has its filter.
        site.show_people(&mut ada, StartingRole::TeamMember, "reported-by", "names")
            .await?;
        let named = cora
            .send("GET", &format!("{GROUPS}?by=reportedBy"), None)
            .await?;
        let listed = ok(&named)?["groups"].as_array().ok_or("no groups")?.clone();
        assert!(
            listed.iter().all(|group| group["filter"].is_string()),
            "{}",
            named.body
        );
        Ok(())
    })
    .await
}
