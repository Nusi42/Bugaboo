//! The canonical dump of a sample: everything that must be the same when the same seed is made
//! again, on either database, and nothing that is random by design (TEST-SAMPLE-DATA).
//!
//! Random by design are the IDs, the password hashes, the secrets and the hashes of the security
//! log (its key is new for every database). The dump leaves them out, and replaces every ID by a
//! label that counts the records of a kind in the order they first show up (`person#3`), so the
//! dump still says which events and log entries are about the same record. No secret is in it.

use bb_app::auth::AuthStore as _;
use bb_core::event::{Actor, DomainEvent, EntityRef};
use bb_core::technical::TechnicalName;
use serde::Serialize;

use crate::error::SampleError;
use crate::generate::Sample;

/// The name under which the dump reads the outbox. It never marks anything as done, so reading
/// the events changes nothing.
const READER: &str = "sample-dump";
/// More than any sample has: the events and entries are read in one go.
const LIMIT: u64 = 1_000_000;

/// The canonical dump.
#[derive(Debug, Serialize)]
pub struct Dump {
    seed: u64,
    start: String,
    people: Vec<DumpPerson>,
    events: Vec<DumpEvent>,
    security_log: Vec<DumpLogEntry>,
}

#[derive(Debug, Serialize)]
struct DumpPerson {
    name: String,
    email: String,
    role: &'static str,
    administrator: bool,
    code_app: bool,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct DumpEvent {
    time: String,
    actor: String,
    channel: &'static str,
    entity: String,
    changes: Vec<DumpChange>,
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct DumpChange {
    field: String,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct DumpLogEntry {
    seq: i64,
    time: String,
    person: Option<String>,
    action: String,
    subject: Option<String>,
    network: Option<String>,
}

/// Replaces IDs by labels in the order they first show up.
#[derive(Default)]
struct Labels {
    seen: Vec<([u8; 16], String)>,
}

impl Labels {
    /// The label of the record `id` of the `kind`; a new record gets the next number of its kind.
    fn label(&mut self, kind: &str, id: [u8; 16]) -> String {
        if let Some((_, label)) = self.seen.iter().find(|(known, _)| *known == id) {
            return label.clone();
        }
        let number = self
            .seen
            .iter()
            .filter(|(_, label)| label.split_once('#').is_some_and(|(of, _)| of == kind))
            .count()
            + 1;
        let label = format!("{kind}#{number}");
        self.seen.push((id, label.clone()));
        label
    }

    fn entity(&mut self, entity: &EntityRef) -> String {
        self.label(entity.kind.as_str(), entity.id.to_bytes())
    }

    fn actor(&mut self, actor: &Actor) -> String {
        match actor {
            Actor::Person(person) => self.label("person", person.to_bytes()),
            Actor::System => "system".to_owned(),
        }
    }
}

impl Sample {
    /// The canonical dump of this sample.
    ///
    /// # Errors
    /// The errors of the database.
    pub async fn dump(&self) -> Result<Dump, SampleError> {
        let mut labels = Labels::default();
        let mut people = Vec::with_capacity(self.people.len());
        for made in &self.people {
            let account = self
                .storage
                .account_by_email(&made.person.email)
                .await?
                .ok_or_else(|| SampleError::Step {
                    step: "dump",
                    reason: "a person of the plan has no account".to_owned(),
                })?;
            // The label counts the person at the moment they first show up in the dump.
            let _ = labels.label("person", account.person.id.to_bytes());
            let code_app = self
                .storage
                .two_step(account.person.id)
                .await?
                .is_some_and(|step| step.confirmed);
            people.push(DumpPerson {
                name: account.person.display_name,
                email: account.person.email,
                role: made.person.role.code(),
                administrator: account.person.administrator,
                code_app,
                created_at: account.person.created_at.to_string(),
            });
        }

        let reader =
            TechnicalName::new(READER).map_err(|error| SampleError::Store(error.to_string()))?;
        let events = self
            .database
            .outbox_after(&reader, LIMIT)
            .await?
            .into_iter()
            .map(|(_, event)| dump_event(&mut labels, &event))
            .collect();
        let security_log = self
            .database
            .security_log_entries(0, LIMIT)
            .await?
            .into_iter()
            .map(|entry| DumpLogEntry {
                seq: entry.seq,
                time: entry.time.to_string(),
                person: entry
                    .person
                    .map(|person| labels.label("person", person.to_bytes())),
                action: entry.action.as_str().to_owned(),
                subject: entry.subject.as_ref().map(|subject| labels.entity(subject)),
                network: entry.network.map(|network| network.to_string()),
            })
            .collect();
        Ok(Dump {
            seed: self.options.seed,
            start: self.options.start.to_string(),
            people,
            events,
            security_log,
        })
    }
}

fn dump_event(labels: &mut Labels, event: &DomainEvent) -> DumpEvent {
    DumpEvent {
        time: event.time.to_string(),
        actor: labels.actor(&event.actor),
        channel: event.channel.code(),
        entity: labels.entity(&event.entity),
        changes: event
            .changes
            .iter()
            .map(|change| DumpChange {
                field: change.field.clone(),
                before: change.before.clone(),
                after: change.after.clone(),
            })
            .collect(),
        reason: event.reason.clone(),
    }
}

impl Dump {
    /// The dump as the text of `dump.json`: the same text for the same dump.
    ///
    /// # Errors
    /// Fails when a value cannot be written as JSON (never, for the values of the dump).
    pub fn to_json(&self) -> Result<String, SampleError> {
        serde_json::to_string_pretty(self)
            .map(|mut text| {
                text.push('\n');
                text
            })
            .map_err(|error| SampleError::Store(error.to_string()))
    }

    /// How many people the dump has.
    #[must_use]
    pub fn people(&self) -> usize {
        self.people.len()
    }

    /// How many domain events the dump has.
    #[must_use]
    pub fn events(&self) -> usize {
        self.events.len()
    }

    /// How many entries of the security log the dump has.
    #[must_use]
    pub fn security_log_entries(&self) -> usize {
        self.security_log.len()
    }

    /// The actions of the security log, in order.
    pub fn security_log_actions(&self) -> impl Iterator<Item = &str> {
        self.security_log.iter().map(|entry| entry.action.as_str())
    }

    /// The (entity, actor) of each event, as labels.
    pub fn event_entities(&self) -> impl Iterator<Item = (&str, &str)> {
        self.events
            .iter()
            .map(|event| (event.entity.as_str(), event.actor.as_str()))
    }
}
