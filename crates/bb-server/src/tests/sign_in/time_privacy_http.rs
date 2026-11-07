//! Who learns what about a person's time, over HTTP on both databases (PRIV-09, ADR-0035 decisions 8
//! and 9): on an issue each person is shown as *Who sees people* allows for the kind *Time
//! entries*, the lengths only to a reader of *Time spent*, and no route of time orders or ranks
//! people.

use bb_app::access::{Permission, StartingRole};
use bb_core::access::FieldLevel;
use bb_testing::{TestResult, each_database};
use serde_json::{Value, json};

use super::fields_http::reader_of;
use super::http::rig_and_app;
use super::issues_site::{ISSUES, Site, key_of, ok};
use super::time_http::MY_ENTRIES;

/// The name the administrator of the tests goes by.
const ADA: &str = "Ada Admin";

#[tokio::test]
async fn priv_09_a_reader_without_names_for_time_entries_sees_someone_and_never_a_name()
-> TestResult {
    each_database("priv_09_someone", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let booked = ada
            .send(
                "POST",
                MY_ENTRIES,
                Some(json!({"issue": issue, "date": "2027-01-14", "minutes": 45})),
            )
            .await?;
        ok(&booked)?;
        let path = format!("{ISSUES}/{issue}/time-entries");
        // A Viewer sees no names of the kind *Time entries* at the start.
        let (mut vera, _) = site.person(StartingRole::Viewer, "vera").await?;
        let seen = vera.send("GET", &path, None).await?;
        let items = ok(&seen)?["items"].clone();
        assert_eq!(items[0]["person"], json!({"shown": "someone"}), "{items}");
        assert!(!seen.body.to_string().contains(ADA), "{}", seen.body);
        // A Team member sees the name.
        let (mut tom, _) = site.person(StartingRole::TeamMember, "tom").await?;
        let named = tom.send("GET", &path, None).await?;
        assert_eq!(ok(&named)?["items"][0]["person"]["name"], ADA);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_09_the_history_shows_others_the_date_of_an_entry_and_never_the_moment() -> TestResult
{
    each_database("priv_09_history_moment", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let booked = ada
            .send(
                "POST",
                MY_ENTRIES,
                Some(json!({"issue": issue, "date": "2027-01-14", "minutes": 45})),
            )
            .await?;
        ok(&booked)?;
        let path = format!("{ISSUES}/{issue}/history");
        let time_item = |reply: &Value| -> Option<Value> {
            reply["items"]
                .as_array()?
                .iter()
                .find(|item| item["kind"] == "time")
                .cloned()
        };
        // The booker sees the hour of their own booking.
        let own = ada.send("GET", &path, None).await?;
        let own = time_item(ok(&own)?).ok_or("Ada sees no time row")?;
        assert_eq!(own["at"], "2027-01-15T08:00:00Z", "{own}");
        assert!(own.get("day").is_none(), "{own}");
        // Somebody else sees the date of the entry, and the moment appears nowhere in the row.
        let (mut tom, _) = site.person(StartingRole::TeamMember, "tom").await?;
        let other = tom.send("GET", &path, None).await?;
        let other = time_item(ok(&other)?).ok_or("Tom sees no time row")?;
        assert_eq!(other["day"], "2027-01-14", "{other}");
        assert_eq!(other["at"], "2027-01-14T00:00:00Z", "{other}");
        assert!(!other.to_string().contains("08:00"), "{other}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_09_a_reader_who_may_not_read_time_spent_gets_no_lengths() -> TestResult {
    each_database("priv_09_no_lengths", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let booked = ada
            .send(
                "POST",
                MY_ENTRIES,
                Some(json!({
                    "issue": issue,
                    "date": "2027-01-14",
                    "minutes": 45,
                    "entry": {"note": "Cleaned the rollers"},
                })),
            )
            .await?;
        ok(&booked)?;
        let path = format!("{ISSUES}/{issue}/time-entries");
        let mut reader = reader_of(
            &site,
            &rig,
            "reader@example.test",
            &[Permission::SeeProjects, Permission::SeeIssues],
            &[("time-spent", None, FieldLevel::Hidden)],
        )
        .await?;
        let seen = reader.send("GET", &path, None).await?;
        let item = ok(&seen)?["items"][0].clone();
        assert!(item.get("minutes").is_none(), "{item}");
        assert!(item.get("note").is_none(), "{item}");
        let text = seen.body.to_string();
        assert!(!text.contains("rollers"), "{text}");
        // Asking for the length by name is like asking for a field that does not exist.
        let asked = reader
            .send("GET", &format!("{path}?fields=id,minutes"), None)
            .await?;
        assert_eq!(asked.code(), "request.unknown-field", "{}", asked.body);
        // A reader of *Time spent* gets the length.
        let (mut vera, _) = site.person(StartingRole::Viewer, "vera").await?;
        let lengths = vera.send("GET", &path, None).await?;
        assert_eq!(ok(&lengths)?["items"][0]["minutes"], 45);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_09_no_time_route_orders_or_ranks_people() -> TestResult {
    let description: Value = serde_json::from_str(bb_api::openapi::json())?;
    let paths = description["paths"]
        .as_object()
        .ok_or("the description has no paths")?;
    let mut time_routes = 0;
    for (path, operations) in paths {
        let Some(operations) = operations.as_object() else {
            continue;
        };
        for (method, operation) in operations {
            let is_time = operation["tags"]
                .as_array()
                .is_some_and(|tags| tags.iter().any(|tag| tag == "time"));
            if !is_time {
                continue;
            }
            time_routes += 1;
            let names: Vec<String> = operation["parameters"]
                .as_array()
                .map(|parameters| {
                    parameters
                        .iter()
                        .filter_map(|parameter| {
                            parameter["name"]
                                .as_str()
                                .or_else(|| parameter["$ref"].as_str())
                                .map(str::to_lowercase)
                        })
                        .collect()
                })
                .unwrap_or_default();
            for name in &names {
                assert!(
                    !["order", "sort", "rank", "top", "group"]
                        .iter()
                        .any(|word| name.contains(word)),
                    "{method} {path} takes `{name}`"
                );
            }
        }
    }
    assert!(time_routes >= 11, "the routes of time are tagged `time`");
    Ok(())
}
