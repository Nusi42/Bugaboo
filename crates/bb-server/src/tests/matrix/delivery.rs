//! The delivery column of the matrix (SEC-MATRIX, M2-F65, NTF-01, NTF-02, SEC-06, QRY-07): what a
//! person is told through the entry points `notifications`, `e-mail` and `search-mail`, for each
//! starting role as the recipient. The routes of the API are run for every actor by `routes`; what the server sends
//! on its own is run here, for the people who can receive it.
//!
//! The scene is the same for every role: an issue the recipient follows, and one change of its
//! status by the administrator. What the recipient is told is compared with what the tables of the
//! starting roles say they may see: the field *Status* (`StartingRole::field_level`) and the
//! author of the change (*Who sees people*, kind *Authors*, `StartingRole::people_level`). The
//! e-mail of an e-mailed search of the recipient's own (`search-mail`) is compared with the fields
//! *Status* and *Title* of the same tables, and a recipient who is switched off gets none.

use bb_api_words::sec_matrix as words;
use bb_app::access::{Permission, StartingRole};
use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::message::{Message, STATUS_DONE};
use bb_core::people_kinds::PeopleKind;
use bb_testing::{Failure, TestDatabase};
use serde_json::{Value, json};

use crate::sign_in_tests::http::{Browser, rig_and_app};
use crate::sign_in_tests::issues_site::{ISSUES, Site, key_of, ok, version_of};
use crate::sign_in_tests::rig::Rig;
use crate::sign_in_tests::search_mail_http::{EVERY_DAY, is_search_mail, save, set};

/// The name of the administrator who makes the change.
pub(super) const CHANGER: &str = "Ada Admin";

/// How long the server waits before it sends the e-mail of a change, plus a little: the clock goes
/// this far before the jobs run.
const AFTER_THE_WAIT: i64 = bb_core::notification::MAIL_BATCH_SECONDS + 1;

/// What one recipient was told through one entry point, in the words of the record: each a name
/// and its word (`heard=yes`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Delivered {
    /// The entry point: one of [`words::DELIVERY_POINTS`].
    pub(super) entry: &'static str,
    /// The starting role of the recipient.
    pub(super) role: StartingRole,
    /// What they were told.
    pub(super) said: Vec<(&'static str, &'static str)>,
}

/// `yes` or `no`, in the words of the record.
const fn yes_no(yes: bool) -> &'static str {
    if yes { words::YES } else { words::NO }
}

/// `shown` or `hidden`, in the words of the record.
const fn shown(shown: bool) -> &'static str {
    if shown { words::SHOWN } else { words::LEFT_OUT }
}

impl Delivered {
    /// What the tables of the starting roles say the recipient is told, when anything reaches them.
    fn expected(entry: &'static str, role: StartingRole) -> Self {
        let sees = |field| shown(role.field_level(field) != FieldLevel::Hidden);
        let said = if entry == words::ENTRY_SEARCH_MAIL {
            vec![
                ("heard", yes_no(follows_issues(role))),
                ("status", sees(Field::Status)),
                ("title", sees(Field::Title)),
                ("switched-off", words::NO),
            ]
        } else {
            vec![
                ("heard", yes_no(follows_issues(role))),
                ("status", sees(Field::Status)),
                ("author", role.people_level(PeopleKind::Authors).code()),
            ]
        };
        Self { entry, role, said }
    }

    /// What was told about a change of the status: whether anything came, whether the status was
    /// in it, and how its author was shown.
    fn of_a_change(
        entry: &'static str,
        role: StartingRole,
        (heard, status, author): (bool, bool, PeopleLevel),
    ) -> Self {
        Self {
            entry,
            role,
            said: vec![
                ("heard", yes_no(heard)),
                ("status", shown(status)),
                ("author", author.code()),
            ],
        }
    }
}

/// Whether a person with the role can follow an issue at all: *HR* sees no project, so it follows
/// nothing and hears of no issue (ADR-0035, decision 8).
pub(super) fn follows_issues(role: StartingRole) -> bool {
    role.permissions().contains(&Permission::SeeProjects)
}

/// A change of the status that the person may follow: the issue is made by the recipient when they
/// cannot see the issues of others (a Guest), and by the administrator otherwise.
async fn issue_of(
    site: &Site<'_>,
    ada: &mut Browser,
    role: StartingRole,
    person: &mut Browser,
) -> Result<String, Failure> {
    let title = format!("Printer of the {}", role.code());
    let made = if role.permissions().contains(&Permission::SeeIssues) {
        site.create(ada, &title).await?
    } else {
        site.create(person, &title).await?
    };
    let key = key_of(&made)?;
    let followed = person
        .send("PUT", &format!("{ISSUES}/{key}/follow"), None)
        .await?;
    assert!(followed.status.is_success(), "{}", followed.body);
    Ok(key)
}

/// How an author showed in a text: by name, by the label of the role, or not at all.
pub(super) fn author_in(text: &str, name: &str, role_label_part: &str) -> PeopleLevel {
    if text.contains(name) {
        PeopleLevel::Names
    } else if text.contains(role_label_part) {
        PeopleLevel::RoleOnly
    } else {
        PeopleLevel::Hidden
    }
}

/// Runs the scene for every starting role and says what each was told through each entry point.
///
/// # Errors
/// The server answers something the scene does not expect.
pub(super) async fn run(db: &TestDatabase) -> Result<Vec<Delivered>, Failure> {
    let (rig, app) = rig_and_app(db).await?;
    let (site, mut ada) = Site::start(&rig, &app).await?;
    switch_mail_on(&rig, &mut ada).await?;
    let mut everyone = Vec::new();
    let mut issues = Vec::new();
    for role in StartingRole::ALL.iter().copied() {
        let (mut person, _) = site.person(role, role.code()).await?;
        mail_me_my_search(&mut person).await?;
        switched_off_with_a_search(&site, &mut ada, role).await?;
        if !follows_issues(role) {
            everyone.push(in_the_app(role, &mut person).await?);
            issues.push((role, None));
            continue;
        }
        let issue = issue_of(&site, &mut ada, role, &mut person).await?;
        rig.clock.advance(5);
        let version = version_of(&mut ada, &issue).await?;
        let changed = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{issue}"),
                Some(json!({"version": version, "status": "done"})),
            )
            .await?;
        assert!(changed.status.is_success(), "{}", changed.body);
        everyone.push(in_the_app(role, &mut person).await?);
        issues.push((role, Some(issue)));
    }
    rig.clock.advance(AFTER_THE_WAIT);
    rig.run_jobs().await?;
    for role in StartingRole::ALL.iter().copied() {
        everyone.push(by_mail(&rig, role));
    }
    send_the_search_mails(&rig).await?;
    let scene: Vec<String> = issues
        .iter()
        .filter_map(|(_, issue)| issue.clone())
        .collect();
    for (role, issue) in &issues {
        everyone.push(by_search_mail(&rig, (*role, issue.as_deref()), &scene));
    }
    Ok(everyone)
}

/// How the title of an issue of the scene stands in the e-mail of an e-mailed search: quoted, with
/// the name of the starting role after it.
const TITLE_LINE: &str = "> Printer of the ";

/// The address of the person of the role who is switched off.
fn switched_off_address(role: StartingRole) -> String {
    format!("{}-off@example.test", role.code())
}

/// The person saves a search for everything they may see and gets it by e-mail every day.
async fn mail_me_my_search(person: &mut Browser) -> Result<(), Failure> {
    let search = save(person, "Everything I see", "").await?;
    ok(&set(person, &search, EVERY_DAY, false).await?)?;
    Ok(())
}

/// A second person of the role sets an e-mailed search and is switched off.
async fn switched_off_with_a_search(
    site: &Site<'_>,
    ada: &mut Browser,
    role: StartingRole,
) -> Result<(), Failure> {
    let (mut person, id) = site.person(role, &format!("{}-off", role.code())).await?;
    mail_me_my_search(&mut person).await?;
    let off = ada
        .send("POST", &format!("/api/v1/people/{id}/switch-off"), None)
        .await?;
    assert!(off.status.is_success(), "{}", off.body);
    Ok(())
}

/// The rounds of the jobs start, as at a start of the server, and the hour of the e-mailed searches
/// passes.
async fn send_the_search_mails(rig: &Rig) -> Result<(), Failure> {
    rig.services
        .ctx(Rig::info())
        .start_daily_jobs()
        .await
        .map_err(|error| error.to_string())?;
    for _ in 0..2 {
        rig.clock.advance(3600);
        rig.run_jobs().await?;
    }
    Ok(())
}

/// The e-mail of the recipient's e-mailed search: the entry point `search-mail`. `issue` is the
/// issue of the scene the recipient may see, if any. A recipient with none (*HR*) is heard when any
/// issue of the scene is in their e-mail, and shown a title when any title of the scene is, so the
/// line catches what leaks to them.
fn by_search_mail(
    rig: &Rig,
    (role, issue): (StartingRole, Option<&str>),
    scene: &[String],
) -> Delivered {
    let address = format!("{}@example.test", role.code());
    let mails = rig.mailer.messages();
    let text = mails
        .iter()
        .find(|mail| mail.to == address && is_search_mail(mail))
        .map(|mail| mail.text.clone())
        .unwrap_or_default();
    let off = switched_off_address(role);
    let off_heard = mails
        .iter()
        .any(|mail| mail.to == off && is_search_mail(mail));
    Delivered {
        entry: words::ENTRY_SEARCH_MAIL,
        role,
        said: vec![
            (
                "heard",
                yes_no(match issue {
                    Some(key) => text.contains(key),
                    None => scene.iter().any(|key| text.contains(key.as_str())),
                }),
            ),
            (
                "status",
                shown(text.contains(&Message::from(STATUS_DONE).render())),
            ),
            (
                "title",
                shown(match issue {
                    Some(_) => text.contains(&format!("{TITLE_LINE}{}", role.code())),
                    None => text.contains(TITLE_LINE),
                }),
            ),
            ("switched-off", yes_no(off_heard)),
        ],
    }
}

/// The people in the app list: the entry points `notifications`.
async fn in_the_app(role: StartingRole, person: &mut Browser) -> Result<Delivered, Failure> {
    let listed = person.send("GET", "/api/v1/notifications", None).await?;
    let items: Vec<Value> = listed.body["items"].as_array().cloned().unwrap_or_default();
    let status_shown = items.iter().any(|item| item["change"]["field"] == "status");
    let text = items.iter().map(Value::to_string).collect::<String>();
    Ok(Delivered::of_a_change(
        words::ENTRY_NOTIFICATIONS,
        role,
        (
            !items.is_empty(),
            status_shown,
            author_in(&text, CHANGER, "Administrator"),
        ),
    ))
}

/// The e-mail the recipient got: the entry point `e-mail`.
fn by_mail(rig: &Rig, role: StartingRole) -> Delivered {
    let address = format!("{}@example.test", role.code());
    let mails = rig.mailer.messages();
    let mail = mails
        .iter()
        .find(|mail| mail.to == address && !is_search_mail(mail));
    let text = mail.map(|mail| mail.text.clone()).unwrap_or_default();
    Delivered::of_a_change(
        words::ENTRY_MAIL,
        role,
        (
            mail.is_some(),
            // The e-mail says the status in words, the same words as the screen (the catalog).
            text.contains(&Message::from(STATUS_DONE).render()),
            author_in(&text, CHANGER, "Administrator"),
        ),
    )
}

/// Everyone is told by e-mail too, and the mail server is there.
async fn switch_mail_on(rig: &Rig, ada: &mut Browser) -> Result<(), Failure> {
    for (key, value) in [
        ("mail.host", json!("mail.example.test")),
        ("mail.from", json!("bugaboo@example.test")),
    ] {
        let put = ada
            .send(
                "PUT",
                &format!("/api/v1/settings/{key}"),
                Some(json!({"value": value})),
            )
            .await?;
        assert!(put.status.is_success(), "{key}: {}", put.body);
    }
    rig.set_organization_switch("mail.enabled", true).await?;
    rig.set_organization_switch("notifications.followed-activity-by-mail", true)
        .await?;
    Ok(())
}

impl Delivered {
    /// The line of the record.
    pub(super) fn line(&self) -> String {
        let said: Vec<String> = self
            .said
            .iter()
            .map(|(name, word)| format!("{name}={word}"))
            .collect();
        format!(
            "delivery {} {} : {}",
            self.entry,
            words::role_actor(self.role.code()),
            said.join(" ")
        )
    }
}

/// What the server sends on its own, to every starting role as the recipient: the app and the
/// e-mail tell each of them what their role may see and nothing more, and the record says so.
///
/// # Errors
/// The server answers something the scene does not expect, or tells a role something else than
/// the tables of the starting roles allow.
pub(super) async fn run_and_compare(db: &TestDatabase) -> Result<Vec<Delivered>, Failure> {
    let delivered = run(db).await?;
    assert_eq!(
        delivered.len(),
        words::DELIVERY_POINTS.len() * StartingRole::ALL.len()
    );
    for told in &delivered {
        assert_eq!(
            told,
            &Delivered::expected(told.entry, told.role),
            "{}",
            told.line()
        );
    }
    Ok(delivered)
}
