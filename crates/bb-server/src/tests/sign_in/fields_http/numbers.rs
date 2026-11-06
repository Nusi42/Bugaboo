//! The numbers of choices over HTTP on both databases (TIME-13, QRY-05): the choices of a field are
//! listed and found by their number, their name and the two written together, the digits of a
//! number are kept (`0410` is not `410`), a search orders and sorts by the order of the choices,
//! and a person who may not read the field gets the answer for a field that is not there.

use super::super::search_groups_http::encoded;
use super::*;

const SEARCH: &str = "/api/v1/search/issues";

/// A field *Budget* of issues with these choices, `(name, number)`; its key and the ID of each
/// choice by its name.
async fn budget(
    ada: &mut Browser,
    choices: &[(&str, Option<&str>)],
) -> Result<(String, Vec<(String, String)>), Failure> {
    let made = create_field(
        ada,
        json!({
            "name": "Budget",
            "kind": "pick-one",
            "applies_to": "issues",
            "choices": choices
                .iter()
                .map(|(name, number)| json!({"name": name, "number": number}))
                .collect::<Vec<_>>(),
        }),
    )
    .await?;
    let ids = made["choices"]
        .as_array()
        .ok_or("no choices")?
        .iter()
        .map(|choice| {
            Ok((
                choice["name"].as_str().ok_or("no name")?.to_owned(),
                id_in(choice)?,
            ))
        })
        .collect::<Result<Vec<_>, Failure>>()?;
    Ok((id_in(&made)?, ids))
}

/// The keys of the issues a search finds, in the order of the answer.
async fn found(who: &mut Browser, text: &str) -> Result<Vec<String>, Failure> {
    let reply = who
        .send(
            "GET",
            &format!("{SEARCH}?q={}&fields=readable_id&limit=50", encoded(text)),
            None,
        )
        .await?;
    Ok(ok(&reply)?["items"]
        .as_array()
        .ok_or("no items")?
        .iter()
        .filter_map(|item| item["readable_id"].as_str().map(str::to_owned))
        .collect())
}

/// Makes one issue for each choice, in the order of `ids`; their keys by the name of the choice.
async fn issue_for_each(
    site: &Site<'_>,
    ada: &mut Browser,
    key: &str,
    ids: &[(String, String)],
) -> Result<Vec<(String, String)>, Failure> {
    let mut made = Vec::new();
    for (name, choice) in ids {
        let reply = create_with(ada, &site.kind, json!({ key: choice })).await?;
        let issue = ok(&reply)?["readable_id"]
            .as_str()
            .ok_or("no key")?
            .to_owned();
        made.push((name.clone(), issue));
    }
    Ok(made)
}

fn issue_of<'a>(made: &'a [(String, String)], name: &str) -> &'a str {
    made.iter()
        .find(|(each, _)| each == name)
        .map_or("", |(_, issue)| issue.as_str())
}

#[tokio::test]
async fn time_13_a_choice_is_found_by_its_number_its_name_and_both_and_digits_are_kept()
-> TestResult {
    each_database("time_13_numbers_search", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (key, ids) = budget(
            &mut ada,
            &[
                ("Sales", Some("5200")),
                ("Overhead", None),
                ("Plain", Some("410")),
                ("Product development", Some("4100")),
                ("Padded", Some("0410")),
            ],
        )
        .await?;
        let made = issue_for_each(&site, &mut ada, &key, &ids).await?;
        let only = |name: &str| vec![issue_of(&made, name).to_owned()];
        for text in [
            "budget is 4100",
            "budget is \"Product development\"",
            "budget is \"4100 Product development\"",
            "budget is 4100",
        ] {
            assert_eq!(
                found(&mut ada, text).await?,
                only("Product development"),
                "{text}"
            );
        }
        // The digits of a number are kept: 0410 is not 410.
        assert_eq!(found(&mut ada, "budget is 0410").await?, only("Padded"));
        assert_eq!(found(&mut ada, "budget is 410").await?, only("Plain"));
        assert_eq!(found(&mut ada, "budget is \"0410\"").await?, only("Padded"));
        // A comparison counts the choices with a number, in the order of their numbers: the choice
        // without one is left out, and is no operand.
        assert_eq!(found(&mut ada, "budget > 4100").await?, only("Sales"));
        assert_eq!(found(&mut ada, "budget >= 5200").await?, only("Sales"));
        assert!(found(&mut ada, "budget > Overhead").await?.is_empty());
        // A comparison goes by the value of the digits: 0410 and 410 are equal in it.
        let mut above = found(&mut ada, "budget > 0410").await?;
        above.sort();
        let mut expected = vec![
            issue_of(&made, "Product development").to_owned(),
            issue_of(&made, "Sales").to_owned(),
        ];
        expected.sort();
        assert_eq!(above, expected);
        assert!(found(&mut ada, "budget < 0410").await?.is_empty());
        let mut equal = found(&mut ada, "budget <= 410").await?;
        equal.sort();
        let mut expected = vec![
            issue_of(&made, "Padded").to_owned(),
            issue_of(&made, "Plain").to_owned(),
        ];
        expected.sort();
        assert_eq!(equal, expected);
        let mut before = found(&mut ada, "budget < 4100").await?;
        before.sort();
        let mut expected = vec![
            issue_of(&made, "Padded").to_owned(),
            issue_of(&made, "Plain").to_owned(),
        ];
        expected.sort();
        assert_eq!(before, expected);
        // Sorting by the field follows the same order: 0410, 410, 4100, 5200, then no number.
        assert_eq!(
            found(&mut ada, "sort by budget").await?,
            [
                issue_of(&made, "Padded"),
                issue_of(&made, "Plain"),
                issue_of(&made, "Product development"),
                issue_of(&made, "Sales"),
                issue_of(&made, "Overhead"),
            ]
        );
        // The catalogue of the search offers the number and the name of each choice.
        let offered = ada.send("GET", "/api/v1/search/fields", None).await?;
        let field = ok(&offered)?["fields"]
            .as_array()
            .or_else(|| offered.body["items"].as_array())
            .ok_or("no fields")?
            .iter()
            .find(|each| each["word"] == "budget")
            .cloned()
            .ok_or("budget is not offered")?;
        let words: Vec<&str> = field["choices"]
            .as_array()
            .ok_or("no choices")?
            .iter()
            .filter_map(Value::as_str)
            .collect();
        for word in [
            "4100",
            "Product development",
            "0410",
            "410",
            "5200",
            "Overhead",
        ] {
            assert!(words.contains(&word), "{word} in {words:?}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_13_a_person_who_may_not_read_the_field_gets_the_answer_for_a_field_that_is_not_there()
-> TestResult {
    each_database("time_13_numbers_hidden", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (key, ids) = budget(&mut ada, &[("Sales", Some("5200"))]).await?;
        issue_for_each(&site, &mut ada, &key, &ids).await?;
        let mut writer = reader_of(
            &site,
            &rig,
            "wes@example.test",
            &[Permission::SeeProjects, Permission::SeeIssues],
            &[("title", None, FieldLevel::CanSee)],
        )
        .await?;
        let search = |text: &str| format!("{SEARCH}?q={}", encoded(text));
        for text in ["budget is 5200", "budget is Sales", "sort by budget"] {
            let hidden = writer.send("GET", &search(text), None).await?;
            let missing = writer
                .send("GET", &search(&text.replace("budget", "nonesuch")), None)
                .await?;
            assert_eq!(hidden.code(), "search.invalid", "{text}: {}", hidden.body);
            assert_eq!(
                hidden.body["mistakes"][0]["key"], missing.body["mistakes"][0]["key"],
                "{text}"
            );
        }
        let offered = writer.send("GET", "/api/v1/search/fields", None).await?;
        assert!(
            !offered.body.to_string().contains("budget"),
            "{}",
            offered.body
        );
        Ok(())
    })
    .await
}
