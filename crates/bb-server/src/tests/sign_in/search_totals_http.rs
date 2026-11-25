//! The totals of a search over HTTP on both databases (QRY-08, PRIV-13, ADR-0050 decision 7): the
//! count and the three lengths by one field, never over an issue or a value the reader may not see,
//! people in the order of their names and merged as *Who sees people* shows them, the groups as
//! pages, and the *Time factor* refused.

use axum::http::StatusCode;
use bb_app::access::{Permission, StartingRole};
use bb_app::roles::SetFieldLevel;
use bb_core::access::FieldLevel;
use bb_core::fields::{Field, FieldKey};
use bb_core::id::ProjectId;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::fields_http::create_field;
use super::http::{Browser, Reply, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, id_in, key_of, ok, version_of};
use super::policy_rig::Member;
use super::rig::Rig;

/// The route of the totals.
pub(super) const TOTALS: &str = "/api/v1/search/totals";

/// The totals `body` asks for, as `who` gets them.
pub(super) async fn totals(who: &mut Browser, body: Value) -> Result<Reply, Failure> {
    who.send("POST", TOTALS, Some(body)).await
}

/// Changes an issue as `who`, on the version it has now.
pub(super) async fn change(who: &mut Browser, issue: &str, mut body: Value) -> TestResult {
    body["version"] = json!(version_of(who, issue).await?);
    ok(&who
        .send("PATCH", &format!("{ISSUES}/{issue}"), Some(body))
        .await?)?;
    Ok(())
}

/// Books `minutes` on an issue as `who`.
async fn book(who: &mut Browser, issue: &str, minutes: u32) -> TestResult {
    ok(&who
        .send(
            "POST",
            "/api/v1/me/time-entries",
            Some(json!({"issue": issue, "date": "2027-01-14", "minutes": minutes})),
        )
        .await?)?;
    Ok(())
}

/// Makes an issue in a project (by its short name) as `who`, with more of its fields; its key.
pub(super) async fn made(
    site: &Site<'_>,
    who: &mut Browser,
    project: &str,
    title: &str,
    more: Value,
) -> Result<String, Failure> {
    let mut body = site.new_issue(title);
    if let (Some(body), Some(more)) = (body.as_object_mut(), more.as_object()) {
        body.extend(more.clone());
    }
    key_of(
        &who.send("POST", &format!("{PROJECTS}/{project}/issues"), Some(body))
            .await?,
    )
}

/// A second project, BETA; its ID.
pub(super) async fn beta(ada: &mut Browser) -> Result<ProjectId, Failure> {
    let made = ada
        .send(
            "POST",
            PROJECTS,
            Some(json!({"short_name": "BETA", "name": "Beta"})),
        )
        .await?;
    Ok(id_in(ok(&made)?)?.parse()?)
}

/// The group of a total whose filter is `filter`.
fn row<'a>(answer: &'a Value, filter: &str) -> Result<&'a Value, Failure> {
    answer["groups"]
        .as_array()
        .and_then(|groups| groups.iter().find(|group| group["filter"] == filter))
        .ok_or_else(|| format!("no group {filter} in {answer}").into())
}

#[tokio::test]
async fn qry_08_totals_group_a_search_by_one_field_with_count_and_three_lengths() -> TestResult {
    each_database("qry_08_totals_lengths", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        for (title, priority, estimate, spent) in [
            ("One", "high", Some(120), 30),
            ("Two", "high", None, 0),
            ("Three", "low", Some(60), 90),
        ] {
            let issue = made(
                &site,
                &mut ada,
                "ACME",
                title,
                json!({"priority": priority}),
            )
            .await?;
            if let Some(estimate) = estimate {
                change(&mut ada, &issue, json!({"estimated_time": estimate})).await?;
            }
            if spent > 0 {
                book(&mut ada, &issue, spent).await?;
            }
        }
        let answer = totals(
            &mut ada,
            json!({"by": "priority", "sums": ["count", "estimated", "time_left", "time_spent"]}),
        )
        .await?;
        let answer = ok(&answer)?;
        assert_eq!(
            answer["sums"],
            json!(["count", "estimated", "time_left", "time_spent"])
        );
        let high = row(answer, "priority is High")?;
        assert_eq!(high["issues"], 2, "{answer}");
        assert_eq!(
            high["sums"],
            json!([
                {"value": 2, "valued": 2},
                {"value": 120, "valued": 1},
                {"value": 90, "valued": 1},
                {"value": 30, "valued": 2},
            ]),
            "{answer}"
        );
        let low = row(answer, "priority is Low")?;
        assert_eq!(
            low["sums"],
            json!([
                {"value": 1, "valued": 1},
                {"value": 60, "valued": 1},
                {"value": 0, "valued": 1},
                {"value": 90, "valued": 1},
            ]),
            "{answer}"
        );
        assert_eq!(answer["all"]["issues"], 3);
        assert_eq!(answer["all"]["sums"][1], json!({"value": 180, "valued": 2}));
        assert!(answer["next_cursor"].is_null());
        // Without a field to group by, only the total of all.
        let all = totals(
            &mut ada,
            json!({"q": "priority is high", "sums": ["time_spent"]}),
        )
        .await?;
        let all = ok(&all)?;
        assert_eq!(all["groups"], json!([]));
        assert_eq!(
            all["all"],
            json!({"issues": 2, "sums": [{"value": 30, "valued": 2}]})
        );
        // A word that names no sum is refused.
        let wrong = totals(&mut ada, json!({"sums": ["average"]})).await?;
        assert_eq!(wrong.status, StatusCode::BAD_REQUEST, "{}", wrong.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_08_the_total_of_all_groups_equals_the_sum_of_the_rows() -> TestResult {
    each_database("qry_08_totals_add_up", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        for (at, priority) in ["high", "low", "critical", "high", "normal"]
            .iter()
            .enumerate()
        {
            let issue = made(
                &site,
                &mut ada,
                "ACME",
                &format!("Issue {at}"),
                json!({"priority": priority}),
            )
            .await?;
            change(&mut ada, &issue, json!({"estimated_time": 15 * (at + 1)})).await?;
        }
        for by in ["priority", "status", "type", "project"] {
            let answer = totals(&mut ada, json!({"by": by, "sums": ["estimated"]})).await?;
            let answer = ok(&answer)?;
            let groups = answer["groups"].as_array().ok_or("no groups")?;
            let issues: u64 = groups
                .iter()
                .filter_map(|each| each["issues"].as_u64())
                .sum();
            let minutes: i64 = groups
                .iter()
                .filter_map(|each| each["sums"][0]["value"].as_i64())
                .sum();
            let valued: u64 = groups
                .iter()
                .filter_map(|each| each["sums"][0]["valued"].as_u64())
                .sum();
            assert_eq!(answer["all"]["issues"], issues, "{by}: {answer}");
            assert_eq!(answer["all"]["sums"][0]["value"], minutes, "{by}: {answer}");
            assert_eq!(answer["all"]["sums"][0]["valued"], valued, "{by}: {answer}");
            assert_eq!(issues, 5, "{by}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_08_a_total_never_includes_an_issue_the_reader_cannot_see() -> TestResult {
    each_database("qry_08_totals_hidden_issue", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        beta(&mut ada).await?;
        let seen = made(&site, &mut ada, "ACME", "Seen", json!({"priority": "high"})).await?;
        let hidden = made(
            &site,
            &mut ada,
            "BETA",
            "Hidden",
            json!({"priority": "high"}),
        )
        .await?;
        change(&mut ada, &seen, json!({"estimated_time": 60})).await?;
        change(&mut ada, &hidden, json!({"estimated_time": 600})).await?;
        book(&mut ada, &hidden, 45).await?;
        let (mut tim, _) = site.person_in_acme(StartingRole::TeamMember, "tim").await?;
        let asked = json!({"by": "priority", "sums": ["estimated", "time_spent"]});
        let answer = totals(&mut tim, asked.clone()).await?;
        let answer = ok(&answer)?;
        let high = row(answer, "priority is High")?;
        assert_eq!(high["issues"], 1, "{answer}");
        assert_eq!(high["sums"][0], json!({"value": 60, "valued": 1}));
        assert_eq!(high["sums"][1], json!({"value": 0, "valued": 1}));
        assert_eq!(answer["all"]["issues"], 1);
        // The administrator sees both.
        let whole = totals(&mut ada, asked).await?;
        assert_eq!(ok(&whole)?["all"]["issues"], 2);
        // A search that finds nothing has a total of nothing.
        let none = totals(
            &mut tim,
            json!({"q": "title contains \"Hidden\"", "by": "priority", "sums": ["estimated"]}),
        )
        .await?;
        let none = ok(&none)?;
        assert_eq!(none["groups"], json!([]));
        assert_eq!(
            none["all"],
            json!({"issues": 0, "sums": [{"value": null, "valued": 0}]})
        );
        Ok(())
    })
    .await
}

/// A person who sees every issue and reads the title and `field` everywhere, but not `field` in
/// the project `hidden_in` (nowhere when it is none).
async fn reader_of_field(
    site: &Site<'_>,
    rig: &Rig,
    email: &str,
    field: FieldKey,
    hidden_in: Option<ProjectId>,
) -> Result<Browser, Failure> {
    let role = site
        .org
        .role(email, &[Permission::SeeProjects, Permission::SeeIssues])
        .await?;
    let levels = [
        (FieldKey::from(Field::Title), None, FieldLevel::CanSee),
        (field, None, FieldLevel::CanSee),
        (field, hidden_in, FieldLevel::Hidden),
    ];
    for (field, project, level) in levels {
        rig.api()
            .set_field_level(
                Rig::info(),
                SetFieldLevel {
                    session_token: site.org.admin.session.clone(),
                    role: role.id,
                    project,
                    issue_type: None,
                    field,
                    level: Some(level),
                },
            )
            .await?;
    }
    let started = rig.person(&site.org.admin.session, email).await?;
    let member = Member {
        id: started.person.id,
        session: started.session_token,
    };
    site.org.give_everywhere(&member, role.id).await?;
    Browser::signed_in_as(site.app, email).await
}

/// An own field *Points* of whole numbers, for issues; its ID.
async fn points(ada: &mut Browser) -> Result<String, Failure> {
    let made = create_field(
        ada,
        json!({"name": "Points", "kind": "whole-number", "applies_to": "issues"}),
    )
    .await?;
    id_in(&made)
}

#[tokio::test]
async fn qry_08_a_number_field_counts_only_where_the_reader_may_read_it_and_says_how_many_gave_a_value()
-> TestResult {
    each_database("qry_08_totals_number_field", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let beta = beta(&mut ada).await?;
        let field = points(&mut ada).await?;
        for (project, title, value) in [
            ("ACME", "A", Some(3)),
            ("ACME", "B", Some(5)),
            ("ACME", "C", None),
            ("BETA", "D", Some(100)),
        ] {
            let more = match value {
                Some(value) => json!({"fields": {field.clone(): value}}),
                None => json!({}),
            };
            made(&site, &mut ada, project, title, more).await?;
        }
        let asked = json!({"by": "project", "sums": ["number:points"]});
        let all = totals(&mut ada, asked.clone()).await?;
        assert_eq!(
            ok(&all)?["all"]["sums"][0],
            json!({"value": 108, "valued": 3}),
            "{}",
            all.body
        );
        // Points are hidden in BETA: its issues are counted, their points are not.
        let key = FieldKey::Own(field.parse()?);
        let mut reader =
            reader_of_field(&site, &rig, "points@example.test", key, Some(beta)).await?;
        let answer = totals(&mut reader, asked).await?;
        let answer = ok(&answer)?;
        assert_eq!(answer["all"]["issues"], 4, "{answer}");
        assert_eq!(answer["all"]["sums"][0], json!({"value": 8, "valued": 2}));
        let in_beta = row(answer, "project is BETA")?;
        assert_eq!(in_beta["issues"], 1);
        assert_eq!(in_beta["sums"][0], json!({"value": null, "valued": 0}));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_08_a_field_the_reader_reads_nowhere_is_refused_like_an_unknown_field() -> TestResult {
    each_database("qry_08_totals_unread_field", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let field = points(&mut ada).await?;
        made(
            &site,
            &mut ada,
            "ACME",
            "A",
            json!({"fields": {field.clone(): 2}}),
        )
        .await?;
        let key = FieldKey::Own(field.parse()?);
        let mut reader = reader_of_field(&site, &rig, "blind@example.test", key, None).await?;
        let hidden = totals(&mut reader, json!({"sums": ["number:points"]})).await?;
        let unknown = totals(&mut reader, json!({"sums": ["number:nothing"]})).await?;
        assert_eq!(hidden.status, StatusCode::BAD_REQUEST, "{}", hidden.body);
        assert_eq!(hidden.status, unknown.status);
        assert_eq!(hidden.body["code"], unknown.body["code"]);
        // So is a length the reader may not read, and a group by a field they may not read.
        let spent = totals(&mut reader, json!({"sums": ["time_spent"]})).await?;
        assert_eq!(spent.status, unknown.status);
        assert_eq!(spent.body["code"], unknown.body["code"]);
        let by_hidden = totals(&mut reader, json!({"by": "points"})).await?;
        let by_unknown = totals(&mut reader, json!({"by": "nothing"})).await?;
        assert_eq!(by_hidden.status, StatusCode::BAD_REQUEST);
        assert_eq!(by_hidden.body["code"], by_unknown.body["code"]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_08_grouping_or_summing_by_time_factor_is_refused() -> TestResult {
    each_database("qry_08_totals_time_factor", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        site.create(&mut ada, "A").await?;
        let word = Field::TimeFactor.search_word();
        let unknown = totals(&mut ada, json!({"by": "nothing"})).await?;
        for body in [
            json!({"by": word}),
            json!({"sums": [format!("number:{word}")]}),
        ] {
            let refused = totals(&mut ada, body.clone()).await?;
            assert_eq!(refused.status, StatusCode::BAD_REQUEST, "{body}");
            assert_eq!(refused.body["code"], unknown.body["code"], "{body}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_08_people_groups_follow_who_sees_people_and_are_in_the_order_of_names() -> TestResult {
    each_database("qry_08_totals_people", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_, zoe) = site
            .person_called(StartingRole::Contributor, "zoe", "Zoe Zander")
            .await?;
        let (_, amy) = site
            .person_called(StartingRole::Contributor, "amy", "Amy Abbot")
            .await?;
        let (mut tim, _) = site
            .person_called(StartingRole::TeamMember, "tim", "Tim Tester")
            .await?;
        // Zoe has three issues, Amy one; one issue is Amy's and Zoe's together.
        for (title, people) in [
            ("One", vec![zoe]),
            ("Two", vec![zoe]),
            ("Three", vec![zoe, amy]),
            ("Four", vec![]),
        ] {
            let issue = made(&site, &mut ada, "ACME", title, json!({})).await?;
            if !people.is_empty() {
                let people: Vec<String> = people.iter().map(ToString::to_string).collect();
                change(&mut ada, &issue, json!({"assigned_to": people})).await?;
            }
        }
        let asked = json!({"by": "assigned to", "sums": ["count"]});
        // By name: in the order of the names, never by size; the group without a value last.
        let named = totals(&mut ada, asked.clone()).await?;
        let named = ok(&named)?;
        let keys: Vec<(Value, Value)> = named["groups"]
            .as_array()
            .ok_or("no groups")?
            .iter()
            .map(|each| (each["key"]["value"]["name"].clone(), each["issues"].clone()))
            .collect();
        assert_eq!(
            keys,
            [
                (json!("Amy Abbot"), json!(1)),
                (json!("Zoe Zander"), json!(3)),
                (Value::Null, json!(1)),
            ],
            "{named}"
        );
        // No order by size for people.
        let ranked = totals(&mut ada, json!({"by": "assigned to", "order": "count"})).await?;
        assert_eq!(ranked.status, StatusCode::BAD_REQUEST, "{}", ranked.body);
        // *Role only*: the two contributors are one group, where the issue of both counts once.
        site.show_people(
            &mut ada,
            StartingRole::TeamMember,
            "assigned-to",
            "role-only",
        )
        .await?;
        let roles = totals(&mut tim, asked.clone()).await?;
        let roles = ok(&roles)?;
        let groups = roles["groups"].as_array().ok_or("no groups")?;
        assert_eq!(groups.len(), 2, "{roles}");
        assert_eq!(groups[0]["key"]["value"]["shown"], "role", "{roles}");
        assert_eq!(groups[0]["issues"], 3, "{roles}");
        assert!(groups[0]["key"]["value"].get("id").is_none());
        assert_eq!(groups[1]["key"]["type"], "empty");
        assert!(!roles.to_string().contains("Zoe"), "{roles}");
        // *Hidden*: one group *Someone*.
        site.show_people(&mut ada, StartingRole::TeamMember, "assigned-to", "hidden")
            .await?;
        let hidden = totals(&mut tim, asked).await?;
        let hidden = ok(&hidden)?;
        let groups = hidden["groups"].as_array().ok_or("no groups")?;
        assert_eq!(groups.len(), 2, "{hidden}");
        assert_eq!(groups[0]["key"]["value"], json!({"shown": "someone"}));
        assert_eq!(groups[0]["issues"], 3);
        assert!(groups[0]["filter"].is_null());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_08_over_200_groups_the_answer_pages_by_the_group_key() -> TestResult {
    each_database("qry_08_totals_pages", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        for at in 0..205 {
            made(
                &site,
                &mut ada,
                "ACME",
                &format!("Issue {at:03}"),
                json!({}),
            )
            .await?;
        }
        let mut seen: Vec<String> = Vec::new();
        let mut cursor = Value::Null;
        let mut pages = 0;
        loop {
            let page = totals(
                &mut ada,
                json!({"by": "title", "sums": ["count"], "cursor": cursor}),
            )
            .await?;
            let page = ok(&page)?;
            pages += 1;
            assert_eq!(
                page["all"]["issues"], 205,
                "every page has the total of all"
            );
            for group in page["groups"].as_array().ok_or("no groups")? {
                seen.push(
                    group["key"]["value"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
                );
            }
            cursor = page["next_cursor"].clone();
            if cursor.is_null() {
                break;
            }
        }
        assert_eq!(pages, 2);
        assert_eq!(seen.len(), 205);
        let mut sorted = seen.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(seen, sorted, "in the order of the keys, none twice");
        // A cursor of another total opens nothing.
        let first = totals(&mut ada, json!({"by": "title"})).await?;
        let cursor = ok(&first)?["next_cursor"].clone();
        let other = totals(
            &mut ada,
            json!({"by": "title", "q": "title contains \"Issue\"", "cursor": cursor}),
        )
        .await?;
        assert_eq!(other.status, StatusCode::BAD_REQUEST, "{}", other.body);
        Ok(())
    })
    .await
}
