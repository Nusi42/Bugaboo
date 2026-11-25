//! More of the totals of a search over HTTP on both databases (QRY-08, PERF-05, AI-21): the
//! statements a grouped total runs do not grow with its groups, the groups by size come as pages,
//! and the tool `search_totals` answers what the person sees, with a cursor sealed like that of
//! the route.

use bb_app::access::StartingRole;
use bb_core::fields::Field;
use bb_testing::{TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, ok};
use super::perf_queries::Counter;
use super::search_totals_http::{beta, change, made, totals};

#[tokio::test]
async fn perf_05_totals_grouped_by_people_run_as_many_statements_for_forty_as_for_five()
-> TestResult {
    each_database("perf_05_totals_people", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut tim, _) = site
            .person_called(StartingRole::TeamMember, "tim", "Tim Tester")
            .await?;
        let add = async |ada: &mut Browser, range: std::ops::Range<u32>| -> TestResult {
            for number in range {
                let (_, person) = site
                    .person_called(
                        StartingRole::Contributor,
                        &format!("person{number}"),
                        &format!("Person {number:02}"),
                    )
                    .await?;
                let issue = made(&site, ada, "ACME", &format!("Issue {number}"), json!({})).await?;
                change(ada, &issue, json!({"assigned_to": [person.to_string()]})).await?;
            }
            Ok(())
        };
        let asked = json!({"by": "assigned to", "sums": ["count", "estimated", "time_spent"]});
        site.show_people(
            &mut ada,
            StartingRole::TeamMember,
            "assigned-to",
            "role-only",
        )
        .await?;
        let mut counter = Counter::on(&rig.database);
        add(&mut ada, 0..5).await?;
        // Warm, then count: by name for the administrator, merged by role for Tim.
        ok(&totals(&mut ada, asked.clone()).await?)?;
        ok(&totals(&mut tim, asked.clone()).await?)?;
        counter.take();
        ok(&totals(&mut ada, asked.clone()).await?)?;
        let named_on_five = counter.take();
        ok(&totals(&mut tim, asked.clone()).await?)?;
        let merged_on_five = counter.take();
        add(&mut ada, 5..40).await?;
        counter.take();
        let named = totals(&mut ada, asked.clone()).await?;
        assert_eq!(ok(&named)?["groups"].as_array().map_or(0, Vec::len), 40);
        let named_on_forty = counter.take();
        let merged = totals(&mut tim, asked).await?;
        assert_eq!(ok(&merged)?["groups"].as_array().map_or(0, Vec::len), 1);
        let merged_on_forty = counter.take();
        assert!(named_on_five > 0, "the counter sees nothing");
        assert_eq!(
            named_on_five, named_on_forty,
            "more people, more statements"
        );
        assert_eq!(
            merged_on_five, merged_on_forty,
            "more people, more statements"
        );
        Ok(())
    })
    .await
}

/// The numbers after `issues: ` in the text an AI app read, in order.
fn issue_counts(text: &str) -> Vec<u64> {
    text.lines()
        .filter_map(|line| line.trim_start().strip_prefix("issues: ")?.parse().ok())
        .collect()
}

#[tokio::test]
async fn ai_21_search_totals_answers_what_the_person_would_see() -> TestResult {
    use super::ai_mcp_rig::{Mcp, ai_on, sign_in_app};

    each_database("ai_21_search_totals", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
        beta(&mut ada).await?;
        for (project, title, priority) in [
            ("ACME", "One", "high"),
            ("ACME", "Two", "low"),
            ("BETA", "Hidden", "high"),
        ] {
            made(
                &site,
                &mut ada,
                project,
                title,
                json!({"priority": priority}),
            )
            .await?;
        }
        let (mut tim, _) = site.person_in_acme(StartingRole::TeamMember, "tim").await?;
        let mcp = Mcp::new(&app);
        let signed = sign_in_app(&mcp, &mut tim, "Claude Code", "see-only", "see-only").await?;
        let asked = json!({"query": "", "group_by": "priority", "sums": ["count", "estimated"]});
        let answer = mcp.call_tool(&signed, "search_totals", asked).await?;
        assert_eq!(
            answer.json["result"]["isError"],
            json!(false),
            "{}",
            answer.json
        );
        let text = answer.json["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        // The same numbers as the person's own route: all of them, then the groups.
        let route = totals(
            &mut tim,
            json!({"by": "priority", "sums": ["count", "estimated"]}),
        )
        .await?;
        let route = ok(&route)?;
        let mut expected = vec![route["all"]["issues"].as_u64().ok_or("no total")?];
        expected.extend(
            route["groups"]
                .as_array()
                .ok_or("no groups")?
                .iter()
                .filter_map(|group| group["issues"].as_u64()),
        );
        // The members of an answer may come in any order: the numbers are compared as a whole.
        let mut read = issue_counts(&text);
        read.sort_unstable();
        expected.sort_unstable();
        assert_eq!(read, expected, "{text}");
        assert_eq!(route["all"]["issues"], 2, "{route}");
        assert!(!text.contains("Hidden"), "{text}");
        // A group by the time factor is refused, as for the person.
        let factor = mcp
            .call_tool(
                &signed,
                "search_totals",
                json!({"query": "", "group_by": Field::TimeFactor.search_word()}),
            )
            .await?;
        assert_eq!(
            factor.json["result"]["isError"],
            json!(true),
            "{}",
            factor.json
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_05_the_groups_of_a_list_by_assigned_to_are_one_statement_like_the_count() -> TestResult
{
    each_database("perf_05_groups_assigned_to", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let groups = "/api/v1/search/issues/groups?by=assignedTo";
        let mut counter = Counter::on(&rig.database);
        let mut statements = Vec::new();
        for range in [0..5, 5..40] {
            for number in range {
                let (_, person) = site
                    .person_called(
                        StartingRole::Contributor,
                        &format!("person{number}"),
                        &format!("Person {number:02}"),
                    )
                    .await?;
                let issue = made(
                    &site,
                    &mut ada,
                    "ACME",
                    &format!("Issue {number}"),
                    json!({}),
                )
                .await?;
                change(
                    &mut ada,
                    &issue,
                    json!({"assigned_to": [person.to_string()]}),
                )
                .await?;
            }
            ok(&ada.send("GET", groups, None).await?)?;
            counter.take();
            ok(&ada.send("GET", groups, None).await?)?;
            let (ran, log) = counter.take_logged();
            // The groups are one statement of the search: no total of all is read for them.
            let grouped = log.iter().filter(|each| each.contains("GROUP BY")).count();
            assert_eq!(grouped, 1, "{log:#?}");
            statements.push(ran);
        }
        assert_eq!(statements[0], statements[1], "more people, more statements");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_08_groups_by_size_come_as_pages_the_biggest_first() -> TestResult {
    each_database("qry_08_totals_pages_by_size", async |db| {
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
        // Two more issues of one title: that group is the biggest.
        for _ in 0..2 {
            made(&site, &mut ada, "ACME", "Issue 100", json!({})).await?;
        }
        let mut seen: Vec<(String, u64)> = Vec::new();
        let mut cursor = Value::Null;
        loop {
            let page = totals(
                &mut ada,
                json!({"by": "title", "order": "count", "cursor": cursor}),
            )
            .await?;
            let page = ok(&page)?;
            for group in page["groups"].as_array().ok_or("no groups")? {
                seen.push((
                    group["key"]["value"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
                    group["issues"].as_u64().unwrap_or_default(),
                ));
            }
            cursor = page["next_cursor"].clone();
            if cursor.is_null() {
                break;
            }
        }
        assert_eq!(seen.len(), 205);
        assert_eq!(seen[0], ("Issue 100".to_owned(), 3));
        // After the biggest, the groups of one issue each in the order of their keys, none twice.
        let rest: Vec<&String> = seen[1..].iter().map(|(title, _)| title).collect();
        let mut sorted = rest.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(rest, sorted);
        Ok(())
    })
    .await
}

/// The value of the line `name:` of the text an AI app read, written on the line itself or fenced
/// on the lines after it.
fn line_value(text: &str, name: &str) -> Option<String> {
    let mut lines = text
        .lines()
        .skip_while(|line| !line.trim_start().starts_with(name));
    let first = lines.next()?.trim_start().strip_prefix(name)?.trim();
    if !first.is_empty() {
        return Some(first.to_owned());
    }
    lines
        .find(|line| !line.trim().is_empty() && !line.starts_with("========"))
        .map(|line| line.trim().to_owned())
}

#[tokio::test]
async fn ai_21_the_cursor_of_search_totals_is_sealed_like_that_of_the_route() -> TestResult {
    use super::ai_mcp_rig::{Mcp, ai_on, sign_in_app};

    each_database("ai_21_search_totals_cursor", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
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
        let mcp = Mcp::new(&app);
        let signed = sign_in_app(&mcp, &mut ada, "Claude Code", "see-only", "see-only").await?;
        let first = mcp
            .call_tool(
                &signed,
                "search_totals",
                json!({"query": "", "group_by": "title"}),
            )
            .await?;
        let text = first.json["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let cursor = line_value(&text, "next_cursor:").ok_or(text.clone())?;
        // The position is not readable in the cursor: it is sealed.
        assert!(!cursor.contains("smaller"), "{cursor}");
        let next = mcp
            .call_tool(
                &signed,
                "search_totals",
                json!({"query": "", "group_by": "title", "cursor": cursor}),
            )
            .await?;
        assert_eq!(
            next.json["result"]["isError"],
            json!(false),
            "{}",
            next.json
        );
        // A changed cursor, or the cursor of another total, opens nothing.
        let mut changed = cursor.clone();
        changed.replace_range(..1, if cursor.starts_with('A') { "B" } else { "A" });
        for (query, cursor) in [("", changed), ("title contains \"Issue\"", cursor)] {
            let refused = mcp
                .call_tool(
                    &signed,
                    "search_totals",
                    json!({"query": query, "group_by": "title", "cursor": cursor}),
                )
                .await?;
            assert_eq!(
                refused.json["result"]["isError"],
                json!(true),
                "{}",
                refused.json
            );
        }
        Ok(())
    })
    .await
}
