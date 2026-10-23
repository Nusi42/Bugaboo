//! What ten people do at once besides editing (M4 WP-10): they move issues through a workflow whose
//! status change asks for who does the work (*Ask for*), put a value in an own field that is hidden
//! from Guests, add files, and read their notifications. A Guest of the run reports an issue of their
//! own and works on it. The people write down what was kept, and `bb-load-check` judges it.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_core::access::FieldLevel;
use bb_files::fixtures::{Format, varied_picture};
use bb_load_check::{Kept, Work};
use bb_testing::Failure;
use serde_json::{Value, json};

use super::super::fields_http::{FIELDS, create_field, set_levels};
use super::super::files_http::upload;
use super::super::http::Browser;
use super::super::issues_site::{Site, id_in, ok, text_in};
use super::super::layouts_http::{AN_EDITOR, LOCKS};
use super::super::rig::Rig;
use super::{ISSUES, Person, Shared, book, version_in};

/// What the own field that Guests cannot see is called.
const HIDDEN_FIELD_NAME: &str = "Internal note";
/// What the people write into that field: no notification of a Guest may carry it.
const HIDDEN_VALUE: &str = "Only for the team";
const NOTIFICATIONS: &str = "/api/v1/notifications";
const WORKFLOWS: &str = "/api/v1/workflows";
/// The side of the pictures people add, in pixels: small, and different for every file.
const PICTURE_SIDE: u32 = 24;

/// Gives the project ACME a workflow of its own: the starting workflow with the status change from
/// *Open* to *In progress* asking for *Assigned to*.
pub(super) async fn workflow_that_asks(
    site: &Site<'_>,
    admin: &mut Browser,
) -> Result<(), Failure> {
    let editor = ok(&admin
        .send("POST", WORKFLOWS, Some(json!({"project": site.acme})))
        .await?)?
    .clone();
    let workflow = text_in(&editor["workflow"], "id")?;
    let lock = ok(&admin
        .send(
            "POST",
            LOCKS,
            Some(json!({
                "what": {"record": {"kind": "workflow", "id": workflow}, "part": "workflow"},
                "editor_id": AN_EDITOR,
            })),
        )
        .await?)?["lock_token"]
        .clone();
    // The rules as the editor shows them; only the change that starts work gets one more.
    let changes: Vec<Value> = editor["changes"]
        .as_array()
        .ok_or("no status changes")?
        .iter()
        .map(|change| {
            let mut rules = change["rules"].clone();
            if change["from"] == "open" && change["to"] == "in-progress" {
                rules["ask_for"] = json!(["assigned-to"]);
            }
            json!({
                "from": change["from"],
                "to": change["to"],
                "button": change["button"],
                "rules": rules,
            })
        })
        .collect();
    let statuses: Vec<Value> = editor["statuses"]
        .as_array()
        .ok_or("no statuses")?
        .iter()
        .map(|status| json!({"key": status["key"], "name": status["name"], "group": status["group"]}))
        .collect();
    let published = admin
        .send(
            "PUT",
            &format!("{WORKFLOWS}/{workflow}"),
            Some(json!({
                "version": editor["workflow"]["version"],
                "lock_token": lock,
                "name": "Asks for who works",
                "statuses": statuses,
                "changes": changes,
            })),
        )
        .await?;
    ok(&published)?;
    Ok(())
}

/// Makes an own field that Guests may not see and says which one it is.
pub(super) async fn field_hidden_from_guests(
    site: &Site<'_>,
    rig: &Rig,
    admin: &mut Browser,
) -> Result<String, Failure> {
    let made = create_field(
        admin,
        json!({"name": HIDDEN_FIELD_NAME, "kind": "text", "applies_to": "issues"}),
    )
    .await?;
    let field = id_in(&made)?;
    let guest = site
        .org
        .starting_roles()
        .await?
        .get(StartingRole::Guest.code())
        .copied()
        .ok_or("no Guest role")?;
    set_levels(site, rig, guest, &[(&field, None, FieldLevel::Hidden)]).await?;
    assert!(
        admin.send("GET", FIELDS, None).await?.status == StatusCode::OK,
        "the fields can be listed"
    );
    Ok(field)
}

/// The issue the Guest reports, as a Work the book names.
pub(super) async fn guests_issue(
    site: &Site<'_>,
    guest: &mut Browser,
    guest_id: &str,
) -> Result<Work, Failure> {
    let made = site.create(guest, "An issue the Guest reported").await?;
    let body = ok(&made)?;
    Ok(Work {
        id: text_in(body, "id")?,
        key: text_in(body, "key")?,
        reporter: Some(guest_id.to_owned()),
    })
}

impl Person {
    /// Moves the issue one status along its workflow, as the buttons of the issue page offer it,
    /// and writes the hidden own field in the same change. A change that asks for who does the work
    /// names the person. When somebody else moved the issue first, the change is not offered any
    /// more: that is the conflict message of a status change, and nothing is half saved.
    pub(super) async fn move_on(
        &mut self,
        work: &Work,
        round: usize,
        hidden_field: &str,
    ) -> Result<(), Failure> {
        let path = format!("{ISSUES}/{}", work.key);
        let read = self.browser.send("GET", &path, None).await?;
        let issue = &ok(&read)?["issue"];
        let version = version_in(&read.body)?;
        let offered = issue["status_changes"].as_array().ok_or("no buttons")?;
        let Some(change) = offered.get((self.number + round) % offered.len().max(1)) else {
            return Ok(());
        };
        let target = text_in(&change["to"], "key")?;
        let mut body = json!({
            "version": version,
            "status": target,
            "fields": { hidden_field: format!("{HIDDEN_VALUE} {}-{round}", self.number) },
        });
        if change["needs"]
            .as_array()
            .is_some_and(|needs| needs.iter().any(|need| need["field"] == "assigned-to"))
        {
            body["assigned_to"] = json!([self.holder]);
        }
        let reply = self.browser.send("PATCH", &path, Some(body)).await?;
        if reply.status == StatusCode::OK {
            book(&self.book)?.kept.push(Kept {
                key: work.key.clone(),
                version: version_in(&reply.body)?,
                status: Some(target),
                ..Kept::default()
            });
            return Ok(());
        }
        // Somebody was first: the issue moved, or changed since it was read. The buttons were
        // read before that, so the change that was pressed can be another one by now, one that
        // asks for who does the work.
        let moved_since = reply.code() == "field.required"
            && version_in(&self.browser.send("GET", &path, None).await?.body)? != version;
        match reply.code() {
            "edit.version-changed" | "workflow.no-such-change" => Ok(()),
            _ if moved_since => Ok(()),
            _ => Err(format!("{}: {}", reply.status, reply.body).into()),
        }
    }

    /// Adds a small picture to the issue; the file is different for every person and round.
    pub(super) async fn add_file(&mut self, work: &Work, round: usize) -> Result<(), Failure> {
        let number = u32::try_from(self.number * 100 + round)?;
        let picture = varied_picture(number, PICTURE_SIDE, PICTURE_SIDE, Format::Png);
        upload(
            &mut self.browser,
            &work.key,
            &format!("picture-{}-{round}.png", self.number),
            &picture,
        )
        .await?;
        book(&self.book)?.files.push(work.key.clone());
        Ok(())
    }

    /// What a Guest reads names no issue but the ones they reported (a notification may be about
    /// none), and shows no field a Guest cannot see: not the name or the value of the own field
    /// the run hid, and not a built-in field that the starting role hides.
    fn check_what_a_guest_reads(&self, items: &[Value]) -> Result<(), Failure> {
        let built_in: Vec<String> = StartingRole::Guest
            .hidden_fields()
            .map(|field| format!("\"{}\"", field.code()))
            .collect();
        for item in items {
            if let Some(key) = item["issue"]["key"].as_str()
                && !self.issues.iter().any(|work| work.key == key)
            {
                return Err(format!("a Guest was shown {key}, which they did not report").into());
            }
            let text = item.to_string();
            let own = [self.hidden_field.as_str(), HIDDEN_FIELD_NAME, HIDDEN_VALUE];
            if let Some(shown) = own
                .iter()
                .copied()
                .chain(built_in.iter().map(String::as_str))
                .find(|hidden| text.contains(hidden))
            {
                return Err(format!("a Guest was shown {shown}: {text}").into());
            }
        }
        Ok(())
    }

    /// Opens the notifications, reads the unread count, and marks what is there as read.
    pub(super) async fn hear(&mut self) -> Result<(), Failure> {
        let listed = self.browser.send("GET", NOTIFICATIONS, None).await?;
        let items = ok(&listed)?["items"].as_array().ok_or("no notifications")?;
        if self.guest {
            self.check_what_a_guest_reads(items)?;
        }
        let ids: Vec<Value> = items
            .iter()
            .take(20)
            .map(|each| each["id"].clone())
            .collect();
        ok(&self
            .browser
            .send("GET", &format!("{NOTIFICATIONS}/unread"), None)
            .await?)?;
        if !ids.is_empty() {
            let marked = self
                .browser
                .send(
                    "POST",
                    &format!("{NOTIFICATIONS}/read"),
                    Some(json!({"ids": ids})),
                )
                .await?;
            if marked.status != StatusCode::NO_CONTENT {
                return Err(format!("{}: {}", marked.status, marked.body).into());
            }
        }
        Ok(())
    }
}

/// The book names the Guest and the field hidden from Guests.
pub(super) fn note_who_is_judged(shared: &Shared, guest: &str, field: &str) -> Result<(), Failure> {
    let mut written = book(shared)?;
    written.guests.push(guest.to_owned());
    written.hidden_from_guests.push(field.to_owned());
    Ok(())
}
