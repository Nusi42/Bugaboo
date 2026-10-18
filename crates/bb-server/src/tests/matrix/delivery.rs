//! The delivery column of the matrix (SEC-MATRIX, M2-F65, NTF-01, NTF-02, SEC-06): what a person
//! is told through the entry points `notifications` and `e-mail`, for each starting role as the
//! recipient. The routes of the API are run for every actor by `routes`; what the server sends
//! on its own is run here, for the people who can receive it.
//!
//! The scene is the same for every role: an issue the recipient follows, and one change of its
//! status by the administrator. What the recipient is told is compared with what the tables of the
//! starting roles say they may see: the field *Status* (`StartingRole::field_level`) and the
//! author of the change (*Who sees people*, kind *Authors*, `StartingRole::people_level`).

use bb_api_words::sec_matrix as words;
use bb_app::access::{Permission, StartingRole};
use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::people_kinds::PeopleKind;
use bb_testing::{Failure, TestDatabase};
use serde_json::{Value, json};

use crate::sign_in_tests::http::{Browser, rig_and_app};
use crate::sign_in_tests::issues_site::{ISSUES, Site, key_of, version_of};
use crate::sign_in_tests::rig::Rig;

/// The name of the administrator who makes the change.
const CHANGER: &str = "Ada Admin";

/// How long the server waits before it sends the e-mail of a change, plus a little: the clock goes
/// this far before the jobs run.
const AFTER_THE_WAIT: i64 = bb_core::notification::MAIL_BATCH_SECONDS + 1;

/// What one recipient was told through one entry point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Delivered {
    /// The entry point: [`words::ENTRY_NOTIFICATIONS`] or [`words::ENTRY_MAIL`].
    pub(super) entry: &'static str,
    /// The starting role of the recipient.
    pub(super) role: StartingRole,
    /// Whether anything reached them.
    pub(super) heard: bool,
    /// Whether the status, the changed field, was in it.
    pub(super) status_shown: bool,
    /// How the author of the change was shown.
    pub(super) author: PeopleLevel,
}

impl Delivered {
    /// What the tables of the starting roles say the recipient is told, when anything reaches them.
    fn expected(entry: &'static str, role: StartingRole) -> Self {
        Self {
            entry,
            role,
            heard: true,
            status_shown: role.field_level(Field::Status) != FieldLevel::Hidden,
            author: role.people_level(PeopleKind::Authors),
        }
    }
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
fn author_in(text: &str, role_label_part: &str) -> PeopleLevel {
    if text.contains(CHANGER) {
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
    for role in StartingRole::ALL.iter().copied() {
        let (mut person, _) = site.person(role, role.code()).await?;
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
    }
    rig.clock.advance(AFTER_THE_WAIT);
    rig.run_jobs().await?;
    for role in StartingRole::ALL.iter().copied() {
        everyone.push(by_mail(&rig, role));
    }
    Ok(everyone)
}

/// The people in the app list: the entry points `notifications`.
async fn in_the_app(role: StartingRole, person: &mut Browser) -> Result<Delivered, Failure> {
    let listed = person.send("GET", "/api/v1/notifications", None).await?;
    let items: Vec<Value> = listed.body["items"].as_array().cloned().unwrap_or_default();
    let status_shown = items.iter().any(|item| item["field"] == "status");
    let text = items.iter().map(Value::to_string).collect::<String>();
    Ok(Delivered {
        entry: words::ENTRY_NOTIFICATIONS,
        role,
        heard: !items.is_empty(),
        status_shown,
        author: author_in(&text, "Administrator"),
    })
}

/// The e-mail the recipient got: the entry point `e-mail`.
fn by_mail(rig: &Rig, role: StartingRole) -> Delivered {
    let address = format!("{}@example.test", role.code());
    let mails = rig.mailer.messages();
    let mail = mails.iter().find(|mail| mail.to == address);
    let text = mail.map(|mail| mail.text.clone()).unwrap_or_default();
    Delivered {
        entry: words::ENTRY_MAIL,
        role,
        heard: mail.is_some(),
        // The e-mail words the status like the screen does: with a capital.
        status_shown: text.to_lowercase().contains("done"),
        author: author_in(&text, "Administrator"),
    }
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
        format!(
            "delivery {} {} : heard={} status={} author={}",
            self.entry,
            words::role_actor(self.role.code()),
            if self.heard { words::YES } else { words::NO },
            if self.status_shown {
                words::SHOWN
            } else {
                words::LEFT_OUT
            },
            self.author.code()
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
