//! *E-mail me this search* over HTTP on SQLite and PostgreSQL (QRY-07, ADR-0033, amendment
//! 2026-11-24): the e-mail goes to the person who set it and nobody else, runs the search with the
//! person's rights when it goes, lists at most 50 issues and counts the rest, follows *Short
//! e-mails*, goes only when something is new or changed, keeps to the hour of the person's time
//! zone across summer time, pauses for a switched-off person and a search that no longer opens,
//! sends nothing while e-mail or the setting is off, and stops at 20 for a person. The e-mailed
//! searches are in *Download my data* and go at *Remove a person*.

use bb_app::access::Permission;
use bb_app::context::Clock as _;
use bb_app::mailer::MailMessage;
use bb_app::search_mail::SearchMailStore as _;
use bb_core::access::FieldLevel;
use bb_core::app_paths;
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{SavedSearchId, UserId};
use bb_core::search_mail::{MailPause, SEARCH_MAIL_ISSUES_MAX, SEARCH_MAILS_MAX};
use bb_policy::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use jiff::Timestamp;
use serde_json::{Value, json};

use super::data_exports_http::my_file;
use super::exit_masks::person_with_levels;
use super::http::{ADA_EMAIL, Browser, Reply, rig_and_app};
use super::issues_site::{ISSUES, Site, key_of, ok, version_of};
use super::removal_http::{remove, switch_off};
use super::rig::Rig;

pub(super) const SEARCHES: &str = "/api/v1/saved-searches";
pub(super) const MAILS: &str = "/api/v1/search-mails";
/// Every day of the week.
pub(super) const EVERY_DAY: [bool; 7] = [true; 7];
pub(super) const HOUR: i64 = 9;
const SECONDS_PER_HOUR: i64 = 3600;

/// Saves a search under `name` as `browser`: its ID.
pub(super) async fn save(browser: &mut Browser, name: &str, text: &str) -> Result<String, Failure> {
    let made = browser
        .send(
            "POST",
            SEARCHES,
            Some(json!({"name": name, "search_text": text})),
        )
        .await?;
    Ok(ok(&made)?["id"]
        .as_str()
        .ok_or("a saved search without ID")?
        .to_owned())
}

/// Sets the e-mailed search of `search` as `browser`: the answer.
pub(super) async fn set(
    browser: &mut Browser,
    search: &str,
    days: [bool; 7],
    only_when_changed: bool,
) -> Result<Reply, Failure> {
    set_at(browser, search, (days, HOUR), only_when_changed).await
}

/// Sets the e-mailed search of `search` as `browser` for the days and the hour of `when`.
pub(super) async fn set_at(
    browser: &mut Browser,
    search: &str,
    (days, hour): ([bool; 7], i64),
    only_when_changed: bool,
) -> Result<Reply, Failure> {
    browser
        .send(
            "PUT",
            &format!("{SEARCHES}/{search}/mail"),
            Some(json!({"days": days, "hour": hour, "only_when_changed": only_when_changed})),
        )
        .await
}

/// Lets `hours` hours pass, one at a time, with the jobs that are due at each.
pub(super) async fn pass_hours(rig: &Rig, hours: i64) -> Result<(), Failure> {
    for _ in 0..hours {
        rig.clock.advance(SECONDS_PER_HOUR);
        rig.run_jobs().await?;
    }
    Ok(())
}

/// Switches e-mail on and starts the rounds of the jobs, as a server does when it starts.
pub(super) async fn start(rig: &Rig) -> Result<(), Failure> {
    rig.turn_mail_on().await?;
    rig.services
        .ctx(Rig::info())
        .start_daily_jobs()
        .await
        .map_err(|error| error.to_string())?;
    rig.run_jobs().await?;
    Ok(())
}

/// The e-mails of e-mailed searches that went to `to`.
pub(super) fn mails_to(rig: &Rig, to: &str) -> Vec<MailMessage> {
    rig.mailer
        .messages()
        .into_iter()
        .filter(|mail| mail.to == to && is_search_mail(mail))
        .collect()
}

/// Whether an e-mail is the e-mail of an e-mailed search: it links to the list of them.
pub(super) fn is_search_mail(mail: &MailMessage) -> bool {
    mail.text.contains(app_paths::NOTIFICATION_SETTINGS)
}

/// The e-mailed search of `search` for `holder`, as stored.
pub(super) async fn stored(
    rig: &Rig,
    holder: UserId,
    search: &str,
) -> Result<bb_core::search_mail::SearchMail, Failure> {
    let search: SavedSearchId = search.parse()?;
    Ok(rig
        .storage
        .search_mail_of(holder, search)
        .await?
        .ok_or("no e-mailed search")?)
}

/// Changes the title of `issue` as `browser`.
pub(super) async fn retitle(
    browser: &mut Browser,
    issue: &str,
    title: &str,
) -> Result<(), Failure> {
    let version = version_of(browser, issue).await?;
    ok(&browser
        .send(
            "PATCH",
            &format!("{ISSUES}/{issue}"),
            Some(json!({"version": version, "title": title})),
        )
        .await?)?;
    Ok(())
}

/// Bob, a team member, saves a search for everything and gets it by e-mail every day at the hour
/// of the tests; an issue is there for it to find. Bob, his ID and the ID of his search.
pub(super) async fn bob_mails_his_search(
    site: &Site<'_>,
    ada: &mut Browser,
) -> Result<(Browser, UserId, String), Failure> {
    let (mut bob, bob_id) = site
        .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
        .await?;
    key_of(&site.create(ada, "Printer jams").await?)?;
    let search = save(&mut bob, "Mine", "").await?;
    ok(&set(&mut bob, &search, EVERY_DAY, false).await?)?;
    Ok((bob, bob_id, search))
}

/// A person with a role of their own that sees the projects and the issues, and of their fields
/// only `fields` (`reader@example.test`).
pub(super) async fn reader_seeing(
    site: &Site<'_>,
    rig: &Rig,
    fields: &[Field],
) -> Result<Browser, Failure> {
    let levels: Vec<_> = fields
        .iter()
        .map(|field| (FieldKey::BuiltIn(*field), None, FieldLevel::CanSee))
        .collect();
    let (reader, _) = person_with_levels(
        site,
        rig,
        "reader",
        &[Permission::SeeProjects, Permission::SeeIssues],
        &levels,
    )
    .await?;
    Ok(reader)
}

pub(super) fn items(list: &Reply) -> Result<Vec<Value>, Failure> {
    Ok(ok(list)?["search_mails"]
        .as_array()
        .cloned()
        .unwrap_or_default())
}

#[tokio::test]
async fn qry_07_a_schedule_sends_the_search_to_the_person_who_set_it_only() -> TestResult {
    each_database("qry_07_to_the_person", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        start(&rig).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let search = save(&mut ada, "Open bugs", "").await?;
        let set = set(&mut ada, &search, EVERY_DAY, false).await?;
        assert_eq!(ok(&set)?["name"], "Open bugs", "{}", set.body);
        assert_eq!(ok(&set)?["hour"], HOUR);
        // Nobody else gets it, and nobody else stops it.
        let theirs = bob
            .send("DELETE", &format!("{SEARCHES}/{search}/mail"), None)
            .await?;
        assert_eq!(theirs.code(), "resource.not-found", "{}", theirs.body);
        assert!(items(&bob.send("GET", MAILS, None).await?)?.is_empty());
        pass_hours(&rig, 3).await?;
        ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let mails = mails_to(&rig, ADA_EMAIL);
        assert_eq!(mails.len(), 1, "{:?}", rig.mailer.messages());
        assert!(mails_to(&rig, "bob@example.test").is_empty());
        let mail = &mails[0];
        assert!(mail.subject.contains("Open bugs"), "{}", mail.subject);
        // The links of Bugaboo come first; the issue with its status and link follows.
        let first = mail.text.lines().next().unwrap_or_default();
        assert!(first.contains(&format!("/s/{search}")), "{}", mail.text);
        assert!(
            mail.text.contains(&format!("/issues/{issue}")),
            "{}",
            mail.text
        );
        assert!(mail.text.contains("> Printer jams"), "{}", mail.text);
        let listed = items(&ada.send("GET", MAILS, None).await?)?;
        assert_eq!(listed.len(), 1);
        assert!(listed[0]["last_sent_at"].is_string(), "{listed:?}");
        // Stopped, nothing more goes.
        let stopped = ada
            .send("DELETE", &format!("{SEARCHES}/{search}/mail"), None)
            .await?;
        assert!(stopped.status.is_success(), "{}", stopped.body);
        pass_hours(&rig, 25).await?;
        assert_eq!(mails_to(&rig, ADA_EMAIL).len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_07_the_search_runs_with_the_persons_rights_at_sending_time() -> TestResult {
    each_database("qry_07_rights_when_sent", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        start(&rig).await?;
        let (mut bob, bob_id) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let early = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let search = save(&mut bob, "Everything", "").await?;
        ok(&set(&mut bob, &search, EVERY_DAY, false).await?)?;
        // An issue made after the e-mailed search was set is found when it goes.
        let later = key_of(&site.create(&mut ada, "Scanner stuck").await?)?;
        pass_hours(&rig, 3).await?;
        ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let mails = mails_to(&rig, "bob@example.test");
        assert_eq!(mails.len(), 1);
        for issue in [&early, &later] {
            assert!(mails[0].text.contains(issue.as_str()), "{}", mails[0].text);
        }
        // Switched off, Bob gets nothing more, whatever his e-mailed search says.
        switch_off(&mut ada, bob_id).await?;
        pass_hours(&rig, 25).await?;
        assert_eq!(mails_to(&rig, "bob@example.test").len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_07_the_e_mail_is_plain_text_with_at_most_50_issues_and_how_many_more() -> TestResult {
    each_database("qry_07_fifty_and_more", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        start(&rig).await?;
        let more = 3;
        for at in 0..SEARCH_MAIL_ISSUES_MAX + more {
            key_of(&site.create(&mut ada, &format!("Issue {at}")).await?)?;
        }
        let search = save(&mut ada, "All", "").await?;
        ok(&set(&mut ada, &search, EVERY_DAY, false).await?)?;
        pass_hours(&rig, 3).await?;
        let mails = mails_to(&rig, ADA_EMAIL);
        let text = &mails.first().ok_or("no e-mail")?.text;
        assert_eq!(
            text.matches("/issues/ACME-").count(),
            SEARCH_MAIL_ISSUES_MAX,
            "{text}"
        );
        assert!(text.contains("And 3 more issues"), "{text}");
        // Plain text: no markup, no control characters but line ends.
        assert!(!text.contains('<') && !text.contains("<html"), "{text}");
        assert!(
            !text.chars().any(|each| each.is_control() && each != '\n'),
            "{text:?}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_07_short_e_mails_leave_out_titles_and_the_search_name() -> TestResult {
    each_database("qry_07_short_e_mails", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        start(&rig).await?;
        rig.set_organization_switch("mail.short", true).await?;
        let issue = key_of(&site.create(&mut ada, "Secret printer plans").await?)?;
        let search = save(&mut ada, "Plans of the board", "").await?;
        ok(&set(&mut ada, &search, EVERY_DAY, false).await?)?;
        let listed = ada.send("GET", MAILS, None).await?;
        assert_eq!(ok(&listed)?["short_e_mails"], true);
        pass_hours(&rig, 3).await?;
        let mails = mails_to(&rig, ADA_EMAIL);
        let mail = mails.first().ok_or("no e-mail")?;
        let whole = format!("{}\n{}", mail.subject, mail.text);
        assert!(whole.contains(issue.as_str()), "{whole}");
        assert!(!whole.contains("Secret printer plans"), "{whole}");
        assert!(!whole.contains("Plans of the board"), "{whole}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_07_nothing_is_sent_when_nothing_is_new_or_changed() -> TestResult {
    each_database("qry_07_only_when_changed", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        start(&rig).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let search = save(&mut ada, "Open bugs", "").await?;
        ok(&set(&mut ada, &search, EVERY_DAY, true).await?)?;
        // The first e-mail goes: nothing was sent before.
        pass_hours(&rig, 3).await?;
        assert_eq!(mails_to(&rig, ADA_EMAIL).len(), 1);
        // A day without a change: nothing.
        pass_hours(&rig, 24).await?;
        ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        assert_eq!(mails_to(&rig, ADA_EMAIL).len(), 1);
        // A change: the next day's e-mail goes.
        retitle(&mut ada, &issue, "Printer jams again").await?;
        pass_hours(&rig, 24).await?;
        ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        assert_eq!(mails_to(&rig, ADA_EMAIL).len(), 2);
        // A new issue counts too.
        key_of(&site.create(&mut ada, "Scanner stuck").await?)?;
        pass_hours(&rig, 24).await?;
        assert_eq!(mails_to(&rig, ADA_EMAIL).len(), 3);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_07_the_schedule_runs_at_the_hour_of_the_persons_time_zone_across_summer_time()
-> TestResult {
    each_database("qry_07_summer_time", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let ada_id = site.org.admin.id;
        ok(&ada
            .send(
                "PUT",
                "/api/v1/me/settings/region.time-zone",
                Some(json!({"value": "Europe/Berlin"})),
            )
            .await?)?;
        // Friday 2027-03-26, noon in UTC: winter time in Berlin until Sunday.
        let friday: Timestamp = "2027-03-26T12:00:00Z".parse()?;
        rig.clock
            .advance(friday.as_second() - rig.clock.now().as_second());
        start(&rig).await?;
        let mut ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let search = save(&mut ada, "Open bugs", "").await?;
        // Saturday and Monday at 9:00 in Berlin.
        let days = [true, false, false, false, false, true, false];
        ok(&set(&mut ada, &search, days, false).await?)?;
        let saturday: Timestamp = "2027-03-27T08:00:00Z".parse()?;
        assert_eq!(stored(&rig, ada_id, &search).await?.next_at, saturday);
        // The round of Saturday moves it to Monday, in summer time (UTC+2).
        pass_hours(&rig, 22).await?;
        let monday: Timestamp = "2027-03-29T07:00:00Z".parse()?;
        assert_eq!(stored(&rig, ada_id, &search).await?.next_at, monday);
        assert_eq!(mails_to(&rig, ADA_EMAIL).len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_07_a_switched_off_person_gets_nothing_and_the_schedule_is_paused() -> TestResult {
    each_database("qry_07_switched_off", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        start(&rig).await?;
        let (_, bob_id, search) = bob_mails_his_search(&site, &mut ada).await?;
        switch_off(&mut ada, bob_id).await?;
        pass_hours(&rig, 3).await?;
        assert!(mails_to(&rig, "bob@example.test").is_empty());
        assert_eq!(
            stored(&rig, bob_id, &search).await?.paused,
            Some(MailPause::SwitchedOff)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_07_a_search_that_no_longer_opens_pauses_the_schedule_and_sends_nothing() -> TestResult
{
    each_database("qry_07_no_longer_opens", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let ada_id = site.org.admin.id;
        start(&rig).await?;
        key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let search = save(&mut ada, "Open bugs", "").await?;
        ok(&set(&mut ada, &search, EVERY_DAY, false).await?)?;
        let trashed = ada
            .send("DELETE", &format!("{SEARCHES}/{search}"), None)
            .await?;
        assert!(trashed.status.is_success(), "{}", trashed.body);
        pass_hours(&rig, 3).await?;
        ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        assert!(mails_to(&rig, ADA_EMAIL).is_empty());
        assert_eq!(
            stored(&rig, ada_id, &search).await?.paused,
            Some(MailPause::NoLongerOpens)
        );
        let listed = items(&ada.send("GET", MAILS, None).await?)?;
        assert_eq!(listed[0]["paused"], "no-longer-opens", "{listed:?}");
        assert_eq!(listed[0]["name"], Value::Null, "{listed:?}");
        // It stays paused when the search is back, until the person sets it again.
        ok(&ada
            .send("POST", &format!("{SEARCHES}/{search}/restore"), None)
            .await?)?;
        pass_hours(&rig, 25).await?;
        ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        assert!(mails_to(&rig, ADA_EMAIL).is_empty());
        ok(&set(&mut ada, &search, EVERY_DAY, false).await?)?;
        pass_hours(&rig, 25).await?;
        assert_eq!(mails_to(&rig, ADA_EMAIL).len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_07_with_e_mail_or_the_setting_off_nothing_is_sent() -> TestResult {
    each_database("qry_07_switches", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        start(&rig).await?;
        key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let search = save(&mut ada, "Open bugs", "").await?;
        ok(&set(&mut ada, &search, EVERY_DAY, false).await?)?;
        rig.set_organization_switch("search-mail.allowed", false)
            .await?;
        let listed = ada.send("GET", MAILS, None).await?;
        assert_eq!(ok(&listed)?["sending"], false);
        pass_hours(&rig, 25).await?;
        rig.set_organization_switch("search-mail.allowed", true)
            .await?;
        rig.set_organization_switch("mail.enabled", false).await?;
        pass_hours(&rig, 25).await?;
        assert!(mails_to(&rig, ADA_EMAIL).is_empty());
        // Both on again: it goes on by itself, nothing waits for the person.
        rig.set_organization_switch("mail.enabled", true).await?;
        pass_hours(&rig, 25).await?;
        assert_eq!(mails_to(&rig, ADA_EMAIL).len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_07_at_most_20_schedules_per_person() -> TestResult {
    each_database("qry_07_twenty", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        for at in 0..SEARCH_MAILS_MAX {
            let search = save(&mut ada, &format!("Search {at}"), "").await?;
            ok(&set(&mut ada, &search, EVERY_DAY, true).await?)?;
            // Setting the same one again changes it and counts once.
            ok(&set(&mut ada, &search, EVERY_DAY, false).await?)?;
        }
        let one_more = save(&mut ada, "One more", "").await?;
        let refused = set(&mut ada, &one_more, EVERY_DAY, true).await?;
        assert_eq!(refused.code(), "limit.reached", "{}", refused.body);
        let listed = ada.send("GET", MAILS, None).await?;
        assert_eq!(items(&listed)?.len(), SEARCH_MAILS_MAX);
        assert_eq!(ok(&listed)?["most"], SEARCH_MAILS_MAX);
        // No weekday, or an hour that is none, is no request.
        let none = set(&mut ada, &one_more, [false; 7], true).await?;
        assert_eq!(none.code(), "request.invalid", "{}", none.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_07_a_field_the_person_may_not_read_never_reaches_the_e_mail() -> TestResult {
    each_database("qry_07_hidden_field", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        start(&rig).await?;
        // The reader sees the status and not the title.
        let mut reader = reader_seeing(&site, &rig, &[Field::Status]).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let search = save(&mut reader, "Mine", "").await?;
        ok(&set(&mut reader, &search, EVERY_DAY, false).await?)?;
        pass_hours(&rig, 3).await?;
        let mails = mails_to(&rig, "reader@example.test");
        let mail = mails.first().ok_or("no e-mail for the reader")?;
        assert!(mail.text.contains(issue.as_str()), "{}", mail.text);
        assert!(!mail.text.contains("Printer jams"), "{}", mail.text);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_removal_deletes_the_persons_schedules() -> TestResult {
    each_database("priv_04_search_mails_go", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let private = save(&mut bob, "Mine", "").await?;
        ok(&set(&mut bob, &private, EVERY_DAY, false).await?)?;
        // An e-mailed search of a search shared with him goes too, and the search stays.
        let shared = save(&mut ada, "Shared", "").await?;
        ok(&ada
            .send(
                "POST",
                &format!("{SEARCHES}/{shared}/shares"),
                Some(json!({"kind": "project", "id": site.acme})),
            )
            .await?)?;
        ok(&set(&mut bob, &shared, EVERY_DAY, false).await?)?;
        assert_eq!(rig.storage.count_search_mails_of(bob_id).await?, 2);
        switch_off(&mut ada, bob_id).await?;
        remove(&mut ada, bob_id, json!({"comments": "keep"})).await?;
        rig.run_jobs().await?;
        assert_eq!(rig.storage.count_search_mails_of(bob_id).await?, 0);
        assert!(
            ok(&ada
                .send("GET", &format!("{SEARCHES}/{shared}"), None)
                .await?)
            .is_ok()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_03_download_my_data_holds_the_persons_schedules() -> TestResult {
    each_database("priv_03_search_mails_in_the_file", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let search = save(&mut ada, "Open bugs", "").await?;
        ok(&set(&mut ada, &search, EVERY_DAY, true).await?)?;
        let mut file = my_file(&rig, &mut ada).await?;
        let rows = file.rows("search_mail_schedule")?;
        assert_eq!(rows.len(), 1, "{:?}", file.names());
        assert!(rows[0].to_string().contains(&search), "{rows:?}");
        Ok(())
    })
    .await
}
