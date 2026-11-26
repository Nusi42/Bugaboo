//! *Download my data* over HTTP (PRIV-03, ADR-0044, decisions 2 and 3): a person asks for the file
//! of their own data, the job writes it, and the person downloads it through the address the
//! server signs for them. Every flow runs on SQLite and PostgreSQL, so the file is made from what
//! the walk really reads from each database.

use std::io::{Cursor, Read as _};

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_files::fixtures::PDF;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};
use zip::ZipArchive;

use super::dashboards_http::{create, results, share, timer};
use super::fields_http::create_field;
use super::files_http::{run_jobs, upload};
use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, id_in, id_of, key_of, ok, text_in, version_of};
use super::plan_privacy_http::field_level;
use super::rig::Rig;

const MY_EXPORT: &str = "/api/v1/me/data-export";

/// The file of a person's data, opened.
pub(crate) struct MyFile {
    zip: ZipArchive<Cursor<Vec<u8>>>,
}

impl MyFile {
    /// The bytes of one entry, if the file has it.
    pub(crate) fn bytes(&mut self, name: &str) -> Result<Option<Vec<u8>>, Failure> {
        let mut entry = match self.zip.by_name(name) {
            Ok(entry) => entry,
            Err(zip::result::ZipError::FileNotFound) => return Ok(None),
            Err(error) => return Err(error.to_string().into()),
        };
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        Ok(Some(bytes))
    }

    /// The rows of one kind of record, as the JSON of the file holds them; none when the file has no
    /// such kind.
    pub(crate) fn rows(&mut self, kind: &str) -> Result<Vec<Value>, Failure> {
        let Some(bytes) = self.bytes(&format!("data/{kind}.json"))? else {
            return Ok(Vec::new());
        };
        let json: Value = serde_json::from_slice(&bytes)?;
        Ok(json
            .get("rows")
            .and_then(Value::as_array)
            .or_else(|| json.as_array())
            .cloned()
            .unwrap_or_default())
    }

    /// The names of the entries of the file.
    pub(crate) fn names(&self) -> Vec<String> {
        self.zip
            .file_names()
            .filter_map(|name| name.ok().map(std::borrow::Cow::into_owned))
            .collect()
    }
}

/// Asks for the file of the person's own data, lets the job write it, and downloads it through
/// the address the server signs for the person.
pub(crate) async fn my_file(rig: &Rig, person: &mut Browser) -> Result<MyFile, Failure> {
    // What waits to be written (the history of a change, say) is written first, so the file holds
    // the same rows on every run.
    run_jobs(rig).await?;
    ok(&person.send("POST", MY_EXPORT, None).await?)?;
    run_jobs(rig).await?;
    let mine = person.send("GET", MY_EXPORT, None).await?;
    assert_eq!(
        ok(&mine)?["export"]["status"],
        "ready",
        "the job wrote the file: {}",
        mine.body
    );
    let export = ok(&mine)?["export"]["id"]
        .as_str()
        .ok_or("no export of the person")?
        .to_owned();
    let address = person
        .send(
            "GET",
            &format!("/api/v1/data-exports/{export}/address"),
            None,
        )
        .await?;
    let url = ok(&address)?["download_url"]
        .as_str()
        .ok_or("no download address")?
        .to_owned();
    let file = person.fetch(&url).await?;
    assert_eq!(file.status, StatusCode::OK, "GET {url}");
    Ok(MyFile {
        zip: ZipArchive::new(Cursor::new(file.bytes)).map_err(|error| error.to_string())?,
    })
}

#[tokio::test]
async fn priv_03_the_files_the_person_uploaded_are_in_the_file_with_their_bytes() -> TestResult {
    each_database("priv_03_uploads_in_the_file", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let ada_file = b"Ada's notes on the printer, as plain text.".to_vec();
        let mine = upload(&mut ada, &issue, "notes.txt", &ada_file).await?;
        // A file somebody else added to the same issue is theirs, not Ada's.
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        let theirs = upload(&mut bob, &issue, "manual.pdf", PDF).await?;
        let mut file = my_file(&rig, &mut ada).await?;
        let id = |attachment: &Value| -> Result<String, Failure> {
            Ok(attachment["id"]
                .as_str()
                .ok_or("an attachment without ID")?
                .to_owned())
        };
        let (mine, theirs) = (id(&mine)?, id(&theirs)?);
        assert_eq!(
            file.bytes(&format!("files/{mine}"))?,
            Some(ada_file),
            "{:?}",
            file.names()
        );
        assert_eq!(file.bytes(&format!("files/{theirs}"))?, None);
        // The JSON names the file by the same ID, so the person can tell which bytes are which.
        let attachments = file.rows("attachment")?;
        assert!(
            attachments
                .iter()
                .any(|row| row.to_string().contains(&mine)),
            "{attachments:?}"
        );
        Ok(())
    })
    .await
}

/// Saves a search as `browser` with the columns of `columns` and one card limit of the board; its
/// ID.
async fn search_with_columns(
    browser: &mut Browser,
    name: &str,
    columns: &[&str],
) -> Result<String, Failure> {
    let columns: Vec<Value> = columns
        .iter()
        .map(|field| json!({"field": field, "width": 240}))
        .collect();
    let made = browser
        .send(
            "POST",
            "/api/v1/saved-searches",
            Some(json!({
                "name": name,
                "search_text": "",
                "presentation": {
                    "view": "board",
                    "columns": columns,
                    "group_by": null,
                    "board": {"columns": null, "rows": null, "card_limits": [{"column": "Open", "limit": 4}]},
                },
            })),
        )
        .await?;
    text_in(ok(&made)?, "id")
}

/// Whether every row of `rows` holds `wanted` and none holds `foreign`.
fn only_theirs(rows: &[Value], wanted: &str, foreign: &str) -> bool {
    rows.iter().all(|row| {
        let row = row.to_string();
        row.contains(wanted) && !row.contains(foreign)
    })
}

#[tokio::test]
async fn priv_03_the_file_holds_the_dashboards_with_their_boxes_and_the_searches_with_their_columns()
-> TestResult {
    each_database("priv_03_parts_in_the_file", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        let adas = id_of(
            &create(
                &mut ada,
                "Ada board",
                vec![
                    results("Ada open bugs", "status is open", (0, 0, 6, 4)),
                    timer(4),
                ],
            )
            .await?,
        )?;
        let adas_search = search_with_columns(&mut ada, "Ada lane", &["title", "status"]).await?;
        // Bob's dashboard is shared with a project Ada sees: she sees it, yet it is his, not hers.
        let bobs = id_of(
            &create(
                &mut bob,
                "Bob board",
                vec![results("Bob open bugs", "status is open", (0, 0, 6, 4))],
            )
            .await?,
        )?;
        ok(&share(&mut bob, &bobs, ("project", &site.acme)).await?)?;
        let bobs_search = search_with_columns(&mut bob, "Bob lane", &["status"]).await?;
        let mut file = my_file(&rig, &mut ada).await?;
        let dashboards = file.rows("dashboard")?;
        assert_eq!(dashboards.len(), 1, "{dashboards:?}");
        assert!(only_theirs(&dashboards, &adas, &bobs), "{dashboards:?}");
        // Each box names its dashboard, so the person can tell which boxes are on which.
        let boxes = file.rows("dashboard_box")?;
        assert_eq!(boxes.len(), 2, "{:?}", file.names());
        assert!(only_theirs(&boxes, &adas, "Bob open bugs"), "{boxes:?}");
        assert!(
            boxes
                .iter()
                .any(|row| row.to_string().contains("Ada open bugs")),
            "{boxes:?}"
        );
        let columns = file.rows("saved_search_column")?;
        assert_eq!(columns.len(), 2, "{:?}", file.names());
        assert!(
            only_theirs(&columns, &adas_search, &bobs_search),
            "{columns:?}"
        );
        let limits = file.rows("saved_search_card_limit")?;
        assert_eq!(limits.len(), 1, "{:?}", file.names());
        assert!(
            only_theirs(&limits, &adas_search, &bobs_search),
            "{limits:?}"
        );
        Ok(())
    })
    .await
}

/// The kind of record of the values of own fields in the file.
const OWN_VALUES: &str = "field_value";

#[tokio::test]
async fn m9_f25_a_value_of_an_own_field_hidden_from_the_person_is_not_in_their_file() -> TestResult
{
    each_database("m9_f25_hidden_own_value", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let reviewer = id_in(
            &create_field(
                &mut ada,
                json!({"name": "Reviewer", "kind": "person", "applies_to": "issues"}),
            )
            .await?,
        )?;
        // Contributors may not see the field in ACME; team members may.
        field_level(
            &site,
            &mut ada,
            StartingRole::Contributor,
            &reviewer,
            "hidden",
        )
        .await?;
        let (mut bob, bob_id) = site.person(StartingRole::TeamMember, "bob").await?;
        let (mut lea, lea_id) = site.person(StartingRole::Contributor, "lea").await?;
        for person in [bob_id, lea_id] {
            let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
            let version = version_of(&mut ada, &issue).await?;
            ok(&ada
                .send(
                    "PATCH",
                    &format!("{ISSUES}/{issue}"),
                    Some(json!({"version": version, "fields": { &reviewer: person.to_string() }})),
                )
                .await?)?;
        }
        // Bob sees the field, so the value that names him is in his file.
        let mut bobs = my_file(&rig, &mut bob).await?;
        assert_eq!(bobs.rows(OWN_VALUES)?.len(), 1, "{:?}", bobs.names());
        // Lea may not see it on screen, so her file holds neither the value nor the kind.
        let leas = my_file(&rig, &mut lea).await?;
        assert!(
            !leas.names().iter().any(|name| name.contains(OWN_VALUES)),
            "{:?}",
            leas.names()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_03_a_persons_data_is_asked_for_a_few_times_a_day_at_most() -> TestResult {
    each_database("priv_03_asks_a_day", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        for ask in 0..bb_core::data_export::EXPORT_ASKS_PER_DAY {
            ok(&ada.send("POST", MY_EXPORT, None).await?)?;
            if ask == 0 {
                // An ask that is refused while one waits counts for nothing.
                let waiting = ada.send("POST", MY_EXPORT, None).await?;
                assert_eq!(waiting.code(), "data-export.waiting", "{}", waiting.body);
            }
            run_jobs(&rig).await?;
        }
        let refused = ada.send("POST", MY_EXPORT, None).await?;
        assert_eq!(
            refused.status,
            StatusCode::TOO_MANY_REQUESTS,
            "{}",
            refused.body
        );
        // The files of the day stay: their records are the count of the asks of the day.
        assert_eq!(
            std::fs::read_dir(rig.folder.path().join(bb_files::EXPORTS_FOLDER))?.count(),
            usize::try_from(bb_core::data_export::EXPORT_ASKS_PER_DAY)?
        );
        Ok(())
    })
    .await
}
