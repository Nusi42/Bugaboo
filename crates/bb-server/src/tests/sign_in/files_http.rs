//! Files over HTTP (ATT-01 … ATT-04, ATT-07, SEC-13, ADR-0032): uploads in parts that go on after
//! an interruption, a file stored once by its content and deleted when nothing points to it, the
//! type read from the content, the limits and blocked types, downloads that are only saved, links
//! for one person and five minutes that are judged again, and previews that a harmful file cannot
//! take down. Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_app::auth::RequestInfo;
use bb_app::files::AttachmentStore as _;
use bb_app::roles::RemoveAssignment;
use bb_core::files::{BIGGEST_FILE_BYTES, MEGABYTE, PART_BYTES, TRASH_KEPT, UNFINISHED_KEPT};
use bb_core::settings::{FILES_BLOCKED_TYPES, FILES_MAX_MEGABYTES};
use bb_files::fixtures::{Format, PDF, picture, pretending_png};
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, RawReply, Reply, rig_and_app};
use super::issues_site::{ISSUES, Site, id_in, key_of, ok};
use super::rig::Rig;

const UPLOADS: &str = "/api/v1/uploads";
const ATTACHMENTS: &str = "/api/v1/attachments";
const DAY: i64 = bb_core::time::SECONDS_PER_DAY;

/// The number of bytes of this many megabytes.
fn megabytes(count: u64) -> usize {
    size_in_bytes(count * MEGABYTE)
}

/// A number of bytes as a size in memory.
fn size_in_bytes(bytes: u64) -> usize {
    usize::try_from(bytes).unwrap_or(usize::MAX)
}

/// A small program, as the first bytes of a Windows program look.
fn a_program() -> Vec<u8> {
    let mut bytes = b"MZ\x90\x00\x03\x00\x00\x00\x04\x00\x00\x00\xff\xff\x00\x00".to_vec();
    bytes.resize(200, 0);
    bytes
}

/// Starts an upload; the answer.
async fn start(
    browser: &mut Browser,
    issue: &str,
    name: &str,
    size: usize,
    comment: Option<&str>,
) -> Result<Reply, Failure> {
    let mut body = json!({"name": name, "size": size});
    if let Some(comment) = comment {
        body["comment"] = json!(comment);
    }
    browser
        .send("POST", &format!("{ISSUES}/{issue}/uploads"), Some(body))
        .await
}

/// Sends a part; the answer.
async fn send_part(
    browser: &mut Browser,
    upload: &str,
    offset: usize,
    bytes: &[u8],
) -> Result<Reply, Failure> {
    browser
        .send_bytes(
            "PUT",
            &format!("{UPLOADS}/{upload}?offset={offset}"),
            bytes.to_vec(),
        )
        .await
}

/// Starts, sends in parts of `part` bytes and finishes; the answer of the end.
async fn upload_in_parts(
    browser: &mut Browser,
    issue: &str,
    name: &str,
    bytes: &[u8],
    part: usize,
    comment: Option<&str>,
) -> Result<Reply, Failure> {
    let started = start(browser, issue, name, bytes.len(), comment).await?;
    if started.status != StatusCode::OK {
        return Ok(started);
    }
    let upload = id_in(&started.body)?;
    for (index, piece) in bytes.chunks(part).enumerate() {
        let sent = send_part(browser, &upload, index * part, piece).await?;
        if sent.status != StatusCode::OK {
            return Ok(sent);
        }
    }
    browser
        .send("POST", &format!("{UPLOADS}/{upload}/finish"), None)
        .await
}

/// A file uploaded in as few parts as there can be; the attachment, which has to be a success.
async fn upload(
    browser: &mut Browser,
    issue: &str,
    name: &str,
    bytes: &[u8],
) -> Result<Value, Failure> {
    let finished = upload_in_parts(browser, issue, name, bytes, PART_BYTES, None).await?;
    Ok(ok(&finished)?.clone())
}

/// The files of an issue.
async fn files(browser: &mut Browser, issue: &str) -> Result<Vec<Value>, Failure> {
    let list = browser
        .send("GET", &format!("{ISSUES}/{issue}/files"), None)
        .await?;
    Ok(ok(&list)?["items"].as_array().cloned().unwrap_or_default())
}

/// The file of this name in a list.
fn named<'a>(items: &'a [Value], name: &str) -> Result<&'a Value, Failure> {
    items
        .iter()
        .find(|item| item["name"] == name)
        .ok_or_else(|| format!("no file {name}").into())
}

/// Every file below a folder of the files, as the disk has them.
fn on_disk(rig: &Rig, folder: &str) -> Vec<std::path::PathBuf> {
    fn walk(folder: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(folder) else {
            return;
        };
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                walk(&entry.path(), found);
            } else {
                found.push(entry.path());
            }
        }
    }
    let mut found = Vec::new();
    walk(
        &rig.folder.path().join(bb_files::FILES_FOLDER).join(folder),
        &mut found,
    );
    found
}

/// Runs the jobs that are due: the previews, and the daily clean-up.
async fn run_jobs(rig: &Rig) -> Result<(), Failure> {
    let ctx = rig.services.ctx(RequestInfo::default());
    ctx.start_daily_jobs().await?;
    while ctx.run_due_jobs().await? > 0 {}
    Ok(())
}

/// A file as the browser fetches it: the answer to the signed link of an attachment.
async fn download(browser: &mut Browser, item: &Value, link: &str) -> Result<RawReply, Failure> {
    browser
        .fetch(item[link].as_str().ok_or("no link in the item")?)
        .await
}

#[tokio::test]
async fn att_02_a_file_is_stored_once_by_its_content_and_deleted_when_nothing_points_to_it()
-> TestResult {
    each_database("att_02_once_by_content", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (first, second) = (
            key_of(&site.create(&mut ada, "Printer jams").await?)?,
            key_of(&site.create(&mut ada, "Printer smokes").await?)?,
        );
        let bytes = b"The printer jams on page two.".to_vec();
        let one = upload(&mut ada, &first, "jam.txt", &bytes).await?;
        let two = upload(&mut ada, &second, "copy of jam.txt", &bytes).await?;
        assert_eq!(
            on_disk(&rig, "content").len(),
            1,
            "the same content is one file"
        );
        assert!(
            on_disk(&rig, "uploads").is_empty(),
            "a finished upload leaves nothing"
        );

        // Both go into the trash; the first twenty days before the second.
        let delete = format!("{ATTACHMENTS}/{}", id_in(&one)?);
        assert_eq!(
            ada.send("DELETE", &delete, None).await?.status,
            StatusCode::NO_CONTENT
        );
        rig.clock.advance(20 * DAY);
        let mut ada = Browser::signed_in_as(&app, super::http::ADA_EMAIL).await?;
        let delete = format!("{ATTACHMENTS}/{}", id_in(&two)?);
        assert_eq!(
            ada.send("DELETE", &delete, None).await?.status,
            StatusCode::NO_CONTENT
        );
        assert!(
            files(&mut ada, &first).await?.is_empty(),
            "a file in the trash is not listed"
        );

        // The first has been in the trash for its days: it is gone, and the file stays for the second.
        rig.clock.advance(11 * DAY);
        run_jobs(&rig).await?;
        assert_eq!(on_disk(&rig, "content").len(), 1);
        assert!(rig.storage.unreferenced_files(10).await?.is_empty());

        // When the second has been in the trash for its days, nothing points to the file, and it goes.
        rig.clock.advance(TRASH_KEPT.as_secs() - 10 * DAY);
        run_jobs(&rig).await?;
        assert!(on_disk(&rig, "content").is_empty());
        assert!(rig.storage.unreferenced_files(10).await?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn att_01_an_interrupted_upload_goes_on_from_the_last_part() -> TestResult {
    each_database("att_01_goes_on", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let bytes = b"0123456789abcdefghijklmnopqrst".to_vec();
        let started = start(&mut ada, &issue, "numbers.txt", bytes.len(), None).await?;
        assert_eq!(ok(&started)?["offset"], 0);
        assert_eq!(started.body["part_bytes"], PART_BYTES);
        let upload = id_in(&started.body)?;

        let first = send_part(&mut ada, &upload, 0, &bytes[..10]).await?;
        assert_eq!(ok(&first)?["offset"], 10);
        // The connection was lost and the page does not know whether the second part arrived: it
        // asks where the server stands.
        let state = ada
            .send("GET", &format!("{UPLOADS}/{upload}"), None)
            .await?;
        assert_eq!(ok(&state)?["offset"], 10);
        // A part that does not start there is refused, and says nothing is lost.
        let again = send_part(&mut ada, &upload, 0, &bytes[..10]).await?;
        assert_eq!(
            (again.status, again.code()),
            (StatusCode::CONFLICT, "file.wrong_part")
        );
        // The file is not complete, so it cannot be finished yet.
        let early = ada
            .send("POST", &format!("{UPLOADS}/{upload}/finish"), None)
            .await?;
        assert_eq!(
            (early.status, early.code()),
            (StatusCode::CONFLICT, "file.incomplete")
        );
        let second = send_part(&mut ada, &upload, 10, &bytes[10..20]).await?;
        assert_eq!(ok(&second)?["offset"], 20);
        let third = send_part(&mut ada, &upload, 20, &bytes[20..]).await?;
        assert_eq!(ok(&third)?["offset"], 30);
        let finished = ada
            .send("POST", &format!("{UPLOADS}/{upload}/finish"), None)
            .await?;
        assert_eq!(ok(&finished)?["size"], 30);
        assert_eq!(finished.body["name"], "numbers.txt");

        // Somebody else cannot see, send to or finish the upload of another person.
        let second_start = start(&mut ada, &issue, "other.txt", 5, None).await?;
        let other = id_in(&second_start.body)?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        for (method, path) in [
            ("GET", format!("{UPLOADS}/{other}")),
            ("POST", format!("{UPLOADS}/{other}/finish")),
            ("DELETE", format!("{UPLOADS}/{other}")),
        ] {
            let refused = bob.send(method, &path, None).await?;
            assert_eq!(refused.status, StatusCode::NOT_FOUND, "{method} {path}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn att_01_an_unfinished_upload_is_deleted_after_a_day_and_only_a_part_has_a_big_body_limit()
-> TestResult {
    each_database("att_01_old_uploads", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let started = start(&mut ada, &issue, "half.bin", 100, None).await?;
        let upload = id_in(&started.body)?;
        send_part(&mut ada, &upload, 0, &[7; 50]).await?;
        assert_eq!(on_disk(&rig, "uploads").len(), 1);
        rig.clock.advance(UNFINISHED_KEPT.as_secs() - 60);
        run_jobs(&rig).await?;
        assert_eq!(on_disk(&rig, "uploads").len(), 1, "not a day yet");
        rig.clock.advance(120);
        run_jobs(&rig).await?;
        assert!(on_disk(&rig, "uploads").is_empty());
        let mut ada = Browser::signed_in_as(&app, super::http::ADA_EMAIL).await?;
        let gone = ada
            .send("GET", &format!("{UPLOADS}/{upload}"), None)
            .await?;
        assert_eq!(gone.status, StatusCode::NOT_FOUND);

        // A part may be as big as a part is, and not a byte more (M1-F30) ...
        let big = start(&mut ada, &issue, "big.bin", PART_BYTES + 1, None).await?;
        let big_upload = id_in(&big.body)?;
        let too_big = send_part(&mut ada, &big_upload, 0, &vec![1; PART_BYTES + 1]).await?;
        assert_eq!(
            (too_big.status, too_big.code()),
            (StatusCode::PAYLOAD_TOO_LARGE, "request.too_large")
        );
        let fits = send_part(&mut ada, &big_upload, 0, &vec![1; PART_BYTES]).await?;
        assert_eq!(ok(&fits)?["offset"], PART_BYTES);
        // ... and every other route keeps the small limit.
        let comment = json!({"text": "x".repeat(2 * 1024 * 1024)});
        let refused = ada
            .send("POST", &format!("{ISSUES}/{issue}/comments"), Some(comment))
            .await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::PAYLOAD_TOO_LARGE, "request.too_large")
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn att_03_the_type_comes_from_the_content_not_the_name() -> TestResult {
    each_database("att_03_content_type", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let png = picture(30, 20, Format::Png);
        let item = upload(&mut ada, &issue, "notes.txt", &png).await?;
        assert_eq!(
            item["media_type"], "image/png",
            "a picture named like a text is a picture"
        );
        let text = upload(&mut ada, &issue, "holiday.png", b"just words").await?;
        assert_eq!(
            text["media_type"], "application/octet-stream",
            "words named like a picture are data"
        );
        let program = upload(&mut ada, &issue, "holiday.jpg", &a_program()).await?;
        assert_eq!(
            program["media_type"],
            "application/vnd.microsoft.portable-executable"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn att_03_a_file_over_the_limit_is_refused_before_it_is_stored() -> TestResult {
    each_database("att_03_limit", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let rules = ada
            .send("GET", &format!("{ISSUES}/{issue}/file-rules"), None)
            .await?;
        assert_eq!(ok(&rules)?["limit_bytes"], 25 * MEGABYTE);
        assert_eq!(rules.body["may_add"], true);
        // 25 MB is the limit, so the byte after it is refused when the upload starts.
        let over = start(&mut ada, &issue, "big.bin", megabytes(25) + 1, None).await?;
        assert_eq!(
            (over.status, over.code()),
            (StatusCode::PAYLOAD_TOO_LARGE, "file.too_big")
        );
        assert_eq!(
            start(&mut ada, &issue, "fits.bin", megabytes(25), None)
                .await?
                .status,
            StatusCode::OK
        );

        // The organization may allow more, up to 100 MB; nobody can set more than that.
        rig.set_organization_setting(FILES_MAX_MEGABYTES.definition().key(), 100)
            .await?;
        let big = start(&mut ada, &issue, "big.bin", megabytes(60), None).await?;
        assert_eq!(big.status, StatusCode::OK);
        let beyond = start(
            &mut ada,
            &issue,
            "bigger.bin",
            size_in_bytes(BIGGEST_FILE_BYTES) + 1,
            None,
        )
        .await?;
        assert_eq!(beyond.code(), "file.too_big");

        // A project may set less than the organization.
        let project = ada
            .send(
                "PUT",
                &format!(
                    "/api/v1/projects/{}/settings/{}",
                    site.acme,
                    FILES_MAX_MEGABYTES.definition().key()
                ),
                Some(json!({"value": 2})),
            )
            .await?;
        assert_eq!(project.status, StatusCode::OK, "{}", project.body);
        let small = start(&mut ada, &issue, "big.bin", megabytes(3), None).await?;
        assert_eq!(small.code(), "file.too_big");
        let rules = ada
            .send("GET", &format!("{ISSUES}/{issue}/file-rules"), None)
            .await?;
        assert_eq!(rules.body["limit_bytes"], 2 * MEGABYTE);

        // Nothing was stored for any of the refused ones; the two that started are only on their way.
        assert!(on_disk(&rig, "content").is_empty());
        // A part cannot go past the size the upload started with.
        let started = start(&mut ada, &issue, "two.bin", 10, None).await?;
        let too_many = send_part(&mut ada, &id_in(&started.body)?, 0, &[0; 11]).await?;
        assert_eq!(too_many.code(), "request.invalid");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn att_03_a_blocked_type_is_refused() -> TestResult {
    each_database("att_03_blocked", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        rig.set_organization_text(FILES_BLOCKED_TYPES.definition().key(), "exe, .BAT")
            .await?;
        // By its name, before anything is sent.
        let named = start(&mut ada, &issue, "Setup.EXE", 10, None).await?;
        assert_eq!(
            (named.status, named.code()),
            (StatusCode::UNSUPPORTED_MEDIA_TYPE, "file.type_blocked")
        );
        // By its content, as soon as the first part shows it, whatever the name says.
        let program = a_program();
        let hidden = upload_in_parts(&mut ada, &issue, "holiday.png", &program, 100, None).await?;
        assert_eq!(
            (hidden.status, hidden.code()),
            (StatusCode::UNSUPPORTED_MEDIA_TYPE, "file.type_blocked")
        );
        assert!(
            on_disk(&rig, "uploads").is_empty(),
            "the refused upload is gone"
        );
        assert!(on_disk(&rig, "content").is_empty());
        // The same bytes are no problem where the type is not blocked.
        rig.set_organization_text(FILES_BLOCKED_TYPES.definition().key(), "")
            .await?;
        assert_eq!(
            upload_in_parts(&mut ada, &issue, "holiday.png", &program, 100, None)
                .await?
                .status,
            StatusCode::OK
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn att_02_svg_and_html_are_only_downloaded_with_attachment_and_nosniff() -> TestResult {
    each_database("att_02_only_downloaded", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(document.cookie)</script></svg>";
        let html = b"<!doctype html><script>alert(document.cookie)</script>";
        for (name, bytes) in [("logo.svg", &svg[..]), ("page.html", &html[..]), ("page.svg.png", &html[..])] {
            let item = upload(&mut ada, &issue, name, bytes).await?;
            assert!(item["preview_url"].is_null(), "{name} has no preview");
            let sent = download(&mut ada, &item, "download_url").await?;
            assert_eq!(sent.status, StatusCode::OK, "{name}");
            assert_eq!(sent.bytes, bytes, "{name}");
            assert!(sent.header("content-disposition").starts_with("attachment;"), "{name}");
            assert!(sent.header("content-disposition").contains(&format!("filename=\"{name}\"")));
            assert_eq!(sent.header("x-content-type-options"), "nosniff");
            assert_eq!(sent.header("content-security-policy"), bb_api::DOWNLOAD_POLICY);
            assert_eq!(sent.header("content-type"), "application/octet-stream");
            assert_eq!(sent.header("cross-origin-resource-policy"), "same-origin");
            assert_eq!(sent.header("cache-control"), "no-store");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn att_04_a_picture_and_a_pdf_get_a_preview_that_is_a_png_of_our_own() -> TestResult {
    each_database("att_04_previews", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let shot = upload(
            &mut ada,
            &issue,
            "screenshot.png",
            &picture(1600, 900, Format::Png),
        )
        .await?;
        let pdf = upload(&mut ada, &issue, "plan.pdf", PDF).await?;
        let notes = upload(&mut ada, &issue, "notes.txt", b"words").await?;
        assert!(
            shot["preview_url"].is_null(),
            "a preview is created by a job, not by the upload"
        );
        run_jobs(&rig).await?;
        let listed = files(&mut ada, &issue).await?;
        assert_eq!(listed.len(), 3);
        for item in &listed {
            let name = item["name"].as_str().unwrap_or_default();
            if name == "notes.txt" {
                assert!(item["preview_url"].is_null());
                continue;
            }
            let sent = download(&mut ada, item, "preview_url").await?;
            assert_eq!(sent.status, StatusCode::OK, "{name}");
            assert_eq!(sent.header("content-type"), "image/png");
            assert!(sent.bytes.starts_with(&[0x89, b'P', b'N', b'G']));
            assert!(
                sent.header("content-disposition")
                    .starts_with("attachment;")
            );
        }
        let _ = (pdf, notes);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn att_04_a_decompression_bomb_gets_no_preview_and_the_server_goes_on() -> TestResult {
    each_database("att_04_bomb", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        // A few bytes that say 100000 by 100000 pixels: it is a file like any other.
        let bomb = pretending_png(100_000, 100_000);
        assert!(bomb.len() < 100);
        let item = upload(&mut ada, &issue, "bomb.png", &bomb).await?;
        assert_eq!(item["media_type"], "image/png");
        run_jobs(&rig).await?;
        let listed = files(&mut ada, &issue).await?;
        assert!(
            named(&listed, "bomb.png")?["preview_url"].is_null(),
            "the bomb has no preview"
        );
        let sent = download(&mut ada, named(&listed, "bomb.png")?, "download_url").await?;
        assert_eq!(sent.bytes, bomb, "it can still be downloaded");
        // The server goes on: the next picture gets its preview.
        upload(&mut ada, &issue, "next.png", &picture(40, 40, Format::Png)).await?;
        run_jobs(&rig).await?;
        let listed = files(&mut ada, &issue).await?;
        assert!(named(&listed, "bomb.png")?["preview_url"].is_null());
        assert!(named(&listed, "next.png")?["preview_url"].is_string());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn att_04_a_broken_pdf_gets_no_preview() -> TestResult {
    each_database("att_04_broken_pdf", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        upload(&mut ada, &issue, "cut.pdf", &PDF[..PDF.len() / 2]).await?;
        upload(&mut ada, &issue, "plan.pdf", PDF).await?;
        run_jobs(&rig).await?;
        let listed = files(&mut ada, &issue).await?;
        assert!(named(&listed, "cut.pdf")?["preview_url"].is_null());
        assert!(named(&listed, "plan.pdf")?["preview_url"].is_string());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn att_07_a_link_is_for_one_person_and_five_minutes() -> TestResult {
    each_database("att_07_one_person", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        upload(&mut ada, &issue, "jam.txt", b"The printer jams.").await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        let (mut cora, _) = site.person(StartingRole::TeamMember, "cora").await?;
        let item = files(&mut bob, &issue).await?.remove(0);
        assert_eq!(
            download(&mut bob, &item, "download_url").await?.status,
            StatusCode::OK
        );
        // Cora sees the issue too, and still the link of Bob opens nothing for her.
        assert_eq!(
            download(&mut cora, &item, "download_url").await?.status,
            StatusCode::NOT_FOUND
        );
        // Five minutes later it has run out; a new list gives a new link.
        rig.clock.advance(5 * 60 - 1);
        assert_eq!(
            download(&mut bob, &item, "download_url").await?.status,
            StatusCode::OK
        );
        rig.clock.advance(1);
        assert_eq!(
            download(&mut bob, &item, "download_url").await?.status,
            StatusCode::NOT_FOUND
        );
        let fresh = files(&mut bob, &issue).await?.remove(0);
        assert_ne!(fresh["download_url"], item["download_url"]);
        assert_eq!(
            download(&mut bob, &fresh, "download_url").await?.status,
            StatusCode::OK
        );
        // A link that was changed, or one that is not one, opens nothing.
        let mut changed = fresh["download_url"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        changed.push('A');
        assert_eq!(bob.fetch(&changed).await?.status, StatusCode::NOT_FOUND);
        assert_eq!(
            bob.fetch("/api/v1/files/nonsense").await?.status,
            StatusCode::NOT_FOUND
        );
        // Not signed in, a link opens nothing either.
        let mut nobody = Browser::new(&app, &super::http::called());
        assert_eq!(
            download(&mut nobody, &fresh, "download_url").await?.status,
            StatusCode::UNAUTHORIZED
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn att_07_a_link_stops_working_after_access_ends() -> TestResult {
    each_database("att_07_access_ends", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        upload(&mut ada, &issue, "jam.txt", b"The printer jams.").await?;
        // Bob has a role in ACME, which Ada takes away again.
        let started = rig
            .person_called(&site.org.admin.session, "bob@example.test", "Bob Builder")
            .await?;
        let bob = super::policy_rig::Member {
            id: started.person.id,
            session: started.session_token,
        };
        let roles = site.org.starting_roles().await?;
        let acme = site.acme.parse().map_err(|_| "no project ID")?;
        let given = site
            .org
            .give(&bob, roles[StartingRole::TeamMember.code()], acme)
            .await?;
        let mut bob = Browser::signed_in_as(&app, "bob@example.test").await?;
        let item = files(&mut bob, &issue).await?.remove(0);
        assert_eq!(
            download(&mut bob, &item, "download_url").await?.status,
            StatusCode::OK
        );

        rig.api()
            .remove_assignment(
                Rig::info(),
                RemoveAssignment {
                    session_token: site.org.admin.session.clone(),
                    assignment: given.id,
                },
            )
            .await?;
        // The link is still within its five minutes, and opens nothing: access is judged again.
        assert_eq!(
            download(&mut bob, &item, "download_url").await?.status,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            bob.send("GET", &format!("{ISSUES}/{issue}/files"), None)
                .await?
                .status,
            StatusCode::NOT_FOUND
        );
        // The same when the issue goes into the trash: Ada's own link stops.
        let item = files(&mut ada, &issue).await?.remove(0);
        assert_eq!(
            download(&mut ada, &item, "download_url").await?.status,
            StatusCode::OK
        );
        let trashed = ada
            .send("DELETE", &format!("{ISSUES}/{issue}"), None)
            .await?;
        assert_eq!(trashed.status, StatusCode::NO_CONTENT, "{}", trashed.body);
        assert_eq!(
            download(&mut ada, &item, "download_url").await?.status,
            StatusCode::NOT_FOUND
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_a_guest_adds_files_only_on_issues_they_reported_and_a_viewer_adds_none()
-> TestResult {
    each_database("sec_06_who_adds_files", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let adas = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let (mut guest, _) = site.person(StartingRole::Guest, "gus").await?;
        let (mut viewer, _) = site.person(StartingRole::Viewer, "vera").await?;
        let (mut contributor, _) = site.person(StartingRole::Contributor, "carl").await?;
        let (mut member, _) = site.person(StartingRole::TeamMember, "mia").await?;

        // A Guest sees, and adds to, only what they reported.
        let guests = key_of(&site.create(&mut guest, "I cannot print").await?)?;
        assert_eq!(
            start(&mut guest, &guests, "shot.png", 10, None)
                .await?
                .status,
            StatusCode::OK
        );
        let hidden = start(&mut guest, &adas, "shot.png", 10, None).await?;
        assert_eq!(
            (hidden.status, hidden.code()),
            (StatusCode::NOT_FOUND, "resource.not_found")
        );
        // A Viewer sees and adds nothing.
        let refused = start(&mut viewer, &adas, "shot.png", 10, None).await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        let rules = viewer
            .send("GET", &format!("{ISSUES}/{adas}/file-rules"), None)
            .await?;
        assert_eq!(rules.body["may_add"], false);
        // A Contributor adds to their own issues, not to those of others; a Team member to any.
        let carls = key_of(&site.create(&mut contributor, "The tray is stuck").await?)?;
        assert_eq!(
            start(&mut contributor, &carls, "tray.png", 10, None)
                .await?
                .status,
            StatusCode::OK
        );
        assert_eq!(
            start(&mut contributor, &adas, "tray.png", 10, None)
                .await?
                .status,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            start(&mut member, &adas, "tray.png", 10, None)
                .await?
                .status,
            StatusCode::OK
        );
        assert_eq!(
            start(&mut member, &guests, "tray.png", 10, None)
                .await?
                .status,
            StatusCode::OK
        );
        // What a person cannot add to, they cannot list when they cannot see it either.
        let listed = guest
            .send("GET", &format!("{ISSUES}/{adas}/files"), None)
            .await?;
        assert_eq!(listed.status, StatusCode::NOT_FOUND);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn att_01_a_deleted_file_goes_to_the_trash_and_undo_brings_it_back() -> TestResult {
    each_database("att_01_trash_and_undo", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let (mut carl, _) = site.person(StartingRole::Contributor, "carl").await?;
        let (mut mia, _) = site.person(StartingRole::TeamMember, "mia").await?;
        let (mut vera, _) = site.person(StartingRole::Viewer, "vera").await?;
        let carls = key_of(&site.create(&mut carl, "The tray is stuck").await?)?;
        let own = upload(&mut carl, &carls, "tray.txt", b"the tray").await?;
        let adas = upload(&mut ada, &issue, "jam.txt", b"the jam").await?;
        assert_eq!(own["may_delete"], true);

        // The person who added a file deletes it; somebody else needs the right to change issues.
        let forbidden = carl
            .send("DELETE", &format!("{ATTACHMENTS}/{}", id_in(&adas)?), None)
            .await?;
        assert_eq!(
            (forbidden.status, forbidden.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        let viewers = vera
            .send("DELETE", &format!("{ATTACHMENTS}/{}", id_in(&adas)?), None)
            .await?;
        assert_eq!(viewers.status, StatusCode::FORBIDDEN);
        assert_eq!(files(&mut vera, &issue).await?[0]["may_delete"], false);
        let gone = carl
            .send("DELETE", &format!("{ATTACHMENTS}/{}", id_in(&own)?), None)
            .await?;
        assert_eq!(gone.status, StatusCode::NO_CONTENT);
        assert!(files(&mut carl, &carls).await?.is_empty());
        // Deleting it twice finds nothing; *Undo* brings it back, for the person who may.
        assert_eq!(
            carl.send("DELETE", &format!("{ATTACHMENTS}/{}", id_in(&own)?), None)
                .await?
                .status,
            StatusCode::NOT_FOUND
        );
        let restore = format!("{ATTACHMENTS}/{}/restore", id_in(&own)?);
        assert_eq!(
            vera.send("POST", &restore, None).await?.status,
            StatusCode::NOT_FOUND
        );
        let back = carl.send("POST", &restore, None).await?;
        assert_eq!(ok(&back)?["name"], "tray.txt");
        assert_eq!(files(&mut carl, &carls).await?.len(), 1);
        // A Team member deletes the file of somebody else.
        let deleted = mia
            .send("DELETE", &format!("{ATTACHMENTS}/{}", id_in(&adas)?), None)
            .await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn att_01_a_file_on_a_comment_is_added_by_its_writer_and_goes_with_the_comment() -> TestResult
{
    each_database("att_01_files_on_comments", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let other = key_of(&site.create(&mut ada, "Printer smokes").await?)?;
        let (mut carl, _) = site.person(StartingRole::Contributor, "carl").await?;
        let written = carl
            .send(
                "POST",
                &format!("{ISSUES}/{issue}/comments"),
                Some(json!({"text": "Here is what I see."})),
            )
            .await?;
        let comment = id_in(ok(&written)?)?;

        // A Contributor may not change Ada's issue, and still adds a file to their own comment.
        assert_eq!(
            start(&mut carl, &issue, "x.png", 10, None).await?.status,
            StatusCode::FORBIDDEN
        );
        let item = ok(&upload_in_parts(
            &mut carl,
            &issue,
            "screen.txt",
            b"what carl sees",
            100,
            Some(&comment),
        )
        .await?)?
        .clone();
        assert_eq!(item["comment"], comment);
        // Nobody else adds files to somebody's comment, not even the administrator.
        let refused = start(&mut ada, &issue, "mine.txt", 10, Some(&comment)).await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        // A comment of another issue is not found.
        let wrong = start(&mut carl, &other, "screen.txt", 10, Some(&comment)).await?;
        assert_eq!(wrong.status, StatusCode::NOT_FOUND);
        // Everybody who sees the issue sees the file under the comment.
        let listed = files(&mut ada, &issue).await?;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0]["comment"], comment);
        assert_eq!(
            download(&mut ada, &listed[0], "download_url").await?.bytes,
            b"what carl sees"
        );

        // The file goes with the comment into the trash, and comes back with it.
        let deleted = carl
            .send("DELETE", &format!("/api/v1/comments/{comment}"), None)
            .await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        assert!(files(&mut ada, &issue).await?.is_empty());
        assert_eq!(
            download(&mut ada, &listed[0], "download_url").await?.status,
            StatusCode::NOT_FOUND
        );
        let restored = carl
            .send("POST", &format!("/api/v1/comments/{comment}/restore"), None)
            .await?;
        assert_eq!(restored.status, StatusCode::OK);
        assert_eq!(files(&mut ada, &issue).await?.len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn att_01_a_person_who_lost_the_right_to_add_a_file_cannot_delete_it_either() -> TestResult {
    each_database("att_01_delete_follows_the_right", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let started = rig
            .person_called(&site.org.admin.session, "bob@example.test", "Bob Builder")
            .await?;
        let bob = super::policy_rig::Member {
            id: started.person.id,
            session: started.session_token,
        };
        let roles = site.org.starting_roles().await?;
        let acme = site.acme.parse().map_err(|_| "no project ID")?;
        let given = site
            .org
            .give(&bob, roles[StartingRole::TeamMember.code()], acme)
            .await?;
        let mut bob = Browser::signed_in_as(&app, "bob@example.test").await?;
        let own = upload(&mut bob, &issue, "bobs.txt", b"what bob saw").await?;
        assert_eq!(own["may_delete"], true);

        // Bob becomes a Viewer: he sees the issue and its files, and may no longer add any.
        rig.api()
            .remove_assignment(
                Rig::info(),
                RemoveAssignment {
                    session_token: site.org.admin.session.clone(),
                    assignment: given.id,
                },
            )
            .await?;
        let member = super::policy_rig::Member {
            id: started.person.id,
            session: site.org.admin.session.clone(),
        };
        site.org
            .give(&member, roles[StartingRole::Viewer.code()], acme)
            .await?;
        // The rule is the one of comments: the writer deletes while they may still write, and
        // whoever may change issues deletes any.
        let listed = files(&mut bob, &issue).await?;
        assert_eq!(named(&listed, "bobs.txt")?["may_delete"], false);
        let refused = bob
            .send("DELETE", &format!("{ATTACHMENTS}/{}", id_in(&own)?), None)
            .await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        let by_ada = ada
            .send("DELETE", &format!("{ATTACHMENTS}/{}", id_in(&own)?), None)
            .await?;
        assert_eq!(by_ada.status, StatusCode::NO_CONTENT);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn att_04_finishing_a_file_that_somebody_else_stored_answers_like_a_new_one() -> TestResult {
    each_database("att_04_same_answer", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let adas = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let (mut mia, _) = site.person(StartingRole::TeamMember, "mia").await?;
        let miras = key_of(&site.create(&mut mia, "The tray is stuck").await?)?;
        let same = picture(40, 30, Format::Png);

        let first = upload(&mut ada, &adas, "shot.png", &same).await?;
        run_jobs(&rig).await?;
        // The preview of the first one is there. The same content, added by somebody who cannot
        // know that: the answer says what it said for the first one, with no preview, no link
        // and no state taken from the other attachment.
        let second = upload(&mut mia, &miras, "shot.png", &same).await?;
        assert_eq!(first["preview"], "pending");
        assert_eq!(second["preview"], first["preview"]);
        assert!(second["preview_url"].is_null());
        assert_eq!(first["preview_url"], second["preview_url"]);
        for member in ["media_type", "size", "may_delete", "name"] {
            assert_eq!(second[member], first[member], "{member}");
        }
        let keys = |item: &Value| -> Vec<String> {
            item.as_object()
                .map(|object| object.keys().cloned().collect())
                .unwrap_or_default()
        };
        assert_eq!(keys(&second), keys(&first));
        // Its own job makes its state ready, and the files are one.
        run_jobs(&rig).await?;
        let listed = files(&mut mia, &miras).await?;
        assert_eq!(listed[0]["preview"], "ready");
        assert_eq!(on_disk(&rig, "content").len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_the_upload_of_somebody_else_cannot_be_reached() -> TestResult {
    each_database("sec_06_upload_of_another", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let (mut carl, _) = site.person(StartingRole::TeamMember, "carl").await?;
        let (mut mia, _) = site.person(StartingRole::TeamMember, "mia").await?;

        let started = start(&mut carl, &issue, "carls.txt", 8, None).await?;
        let upload = id_in(ok(&started)?)?;
        assert_eq!(
            send_part(&mut carl, &upload, 0, b"first").await?.status,
            StatusCode::OK
        );
        // Somebody who may add files to the issue as well, and the administrator, find no such
        // upload: not to read it, not to add a part, not to finish it, not to throw it away.
        for who in [&mut mia, &mut ada] {
            let state = who
                .send("GET", &format!("{UPLOADS}/{upload}"), None)
                .await?;
            assert_eq!(
                (state.status, state.code()),
                (StatusCode::NOT_FOUND, "resource.not_found")
            );
            let part = send_part(who, &upload, 5, b"hijack").await?;
            assert_eq!(part.status, StatusCode::NOT_FOUND);
            let finished = who
                .send("POST", &format!("{UPLOADS}/{upload}/finish"), None)
                .await?;
            assert_eq!(finished.status, StatusCode::NOT_FOUND);
            let thrown = who
                .send("DELETE", &format!("{UPLOADS}/{upload}"), None)
                .await?;
            assert_eq!(thrown.status, StatusCode::NOT_FOUND);
        }
        // Nothing of that touched it: its owner goes on from where it stood and finishes.
        let state = carl
            .send("GET", &format!("{UPLOADS}/{upload}"), None)
            .await?;
        assert_eq!(ok(&state)?["offset"], 5);
        assert_eq!(
            send_part(&mut carl, &upload, 5, b"end").await?.status,
            StatusCode::OK
        );
        let finished = carl
            .send("POST", &format!("{UPLOADS}/{upload}/finish"), None)
            .await?;
        assert_eq!(ok(&finished)?["name"], "carls.txt");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_an_upload_ends_when_the_person_loses_access_in_the_middle_of_it() -> TestResult {
    each_database("sec_06_access_lost_midway", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let created = rig
            .person_called(&site.org.admin.session, "bob@example.test", "Bob Builder")
            .await?;
        let bob = super::policy_rig::Member {
            id: created.person.id,
            session: created.session_token,
        };
        let roles = site.org.starting_roles().await?;
        let acme = site.acme.parse().map_err(|_| "no project ID")?;
        let given = site
            .org
            .give(&bob, roles[StartingRole::TeamMember.code()], acme)
            .await?;
        let mut bob = Browser::signed_in_as(&app, "bob@example.test").await?;
        let started = start(&mut bob, &issue, "bobs.txt", 8, None).await?;
        let upload = id_in(ok(&started)?)?;
        assert_eq!(
            send_part(&mut bob, &upload, 0, b"first").await?.status,
            StatusCode::OK
        );
        assert_eq!(on_disk(&rig, "uploads").len(), 1);

        // Bob loses his role. The next part is refused as for something that is not there, and
        // the upload ends: its bytes and its row are gone, also if the role comes back.
        rig.api()
            .remove_assignment(
                Rig::info(),
                RemoveAssignment {
                    session_token: site.org.admin.session.clone(),
                    assignment: given.id,
                },
            )
            .await?;
        let refused = send_part(&mut bob, &upload, 5, b"end").await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::NOT_FOUND, "resource.not_found")
        );
        assert!(on_disk(&rig, "uploads").is_empty());
        let member = super::policy_rig::Member {
            id: created.person.id,
            session: site.org.admin.session.clone(),
        };
        site.org
            .give(&member, roles[StartingRole::TeamMember.code()], acme)
            .await?;
        let again = bob
            .send("GET", &format!("{UPLOADS}/{upload}"), None)
            .await?;
        assert_eq!(again.status, StatusCode::NOT_FOUND);
        Ok(())
    })
    .await
}
