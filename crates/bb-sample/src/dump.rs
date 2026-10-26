//! The canonical dump of a sample: everything that must be the same when the same seed is made
//! again, on either database, and nothing that is random by design (TEST-SAMPLE-DATA).
//!
//! Random by design are the IDs, the password hashes, the secrets and the hashes of the security
//! log (its key is new for every database). The dump leaves them out, and replaces every ID by a
//! label that counts the records of a kind in the order they first show up (`person#3`), so the
//! dump still says which events and log entries are about the same record. No secret is in it.

use std::collections::{BTreeSet, HashMap};

use bb_app::access::{PolicyStore as _, StartingRole};
use bb_app::auth::{AccountStore as _, SecondStepStore as _};
use bb_core::event::{Actor, DomainEvent, EntityRef};
use bb_core::fields::Field;
use bb_core::id::{EntityId, UserId};
use bb_core::technical::TechnicalName;
use serde::Serialize;

use crate::dump_issues::DumpIssues;
use crate::dump_organization::{DumpAssignment, DumpGroup, DumpLevel, DumpProject, DumpRole};
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
    groups: Vec<DumpGroup>,
    roles: Vec<DumpRole>,
    projects: Vec<DumpProject>,
    assignments: Vec<DumpAssignment>,
    levels: Vec<DumpLevel>,
    issues: DumpIssues,
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
    access_key: bool,
    time_zone: String,
    date_format: String,
    number_format: String,
    first_day: String,
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
    seen: HashMap<[u8; 16], String>,
    /// How many records of each kind have a label.
    counted: HashMap<String, usize>,
}

impl Labels {
    /// The label of the record `id` of the `kind`; a new record gets the next number of its kind.
    fn label(&mut self, kind: &str, id: [u8; 16]) -> String {
        if let Some(label) = self.seen.get(&id) {
            return label.clone();
        }
        let number = self.counted.entry(kind.to_owned()).or_default();
        *number += 1;
        let label = format!("{kind}#{number}");
        self.seen.insert(id, label.clone());
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
    /// The people who have the starting role *Administrator* for the whole organization.
    async fn administrators(&self) -> Result<BTreeSet<UserId>, SampleError> {
        let roles: BTreeSet<_> = self
            .storage
            .roles()
            .await?
            .into_iter()
            .filter(|record| {
                record.role.starting.as_ref().map(TechnicalName::as_str)
                    == Some(StartingRole::Administrator.code())
            })
            .map(|record| record.role.id)
            .collect();
        Ok(self
            .storage
            .assignments_on(None)
            .await?
            .into_iter()
            .filter(|row| row.role.is_some_and(|role| roles.contains(&role)))
            .filter_map(|row| row.person)
            .collect())
    }

    /// The canonical dump of this sample.
    ///
    /// # Errors
    /// The errors of the database.
    pub async fn dump(&self) -> Result<Dump, SampleError> {
        let mut labels = Labels::default();
        let administrators = self.administrators().await?;
        let mut people = Vec::with_capacity(self.people.len());
        for made in &self.people {
            let account = self
                .storage
                .account_by_email(&made.person.email)
                .await?
                .ok_or(SampleError::PersonMissing)?;
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
                administrator: administrators.contains(&account.person.id),
                code_app,
                access_key: made.access_key.is_some(),
                time_zone: made.person.region.time_zone.clone(),
                date_format: made.person.region.date_format.clone(),
                number_format: made.person.region.number_format.clone(),
                first_day: made.person.region.first_day.clone(),
                created_at: account.person.created_at.to_string(),
            });
        }

        let raw: Vec<DomainEvent> = self
            .database
            .outbox_events(READER, LIMIT)
            .await?
            .into_iter()
            .map(|(_, event)| event)
            .collect();
        let issues = self.issues(&raw).await?;
        let events = raw
            .iter()
            .map(|event| dump_event(&mut labels, event))
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
        let organization = self.organization().await?;
        Ok(Dump {
            seed: self.options.seed,
            start: self.options.start.to_string(),
            people,
            groups: organization.groups,
            roles: organization.roles,
            projects: organization.projects,
            assignments: organization.assignments,
            levels: organization.levels,
            issues,
            events,
            security_log,
        })
    }
}

/// The bytes of an ID written the way IDs are written (any kind of ID, so any UUID version).
fn id_bytes(text: &str) -> Option<[u8; 16]> {
    text.parse::<EntityId>().ok().map(EntityId::to_bytes)
}

/// A value of a change, with an ID in it (the role of a person, say) replaced by its label, as
/// the IDs are random by design.
fn labelled(labels: &mut Labels, field: &str, value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(text) => match id_bytes(&text) {
            Some(id) => serde_json::Value::String(labels.label(field, id)),
            None => serde_json::Value::String(text),
        },
        serde_json::Value::Array(items) => serde_json::Value::Array(
            items
                .into_iter()
                .map(|item| labelled(labels, field, item))
                .collect(),
        ),
        // A value made of named parts (the places a workflow is used at): each part is labelled as
        // the kind of record its name says.
        serde_json::Value::Object(parts) => serde_json::Value::Object(
            parts
                .into_iter()
                .map(|(name, part)| {
                    let part = labelled(labels, &name, part);
                    (name, part)
                })
                .collect(),
        ),
        other => other,
    }
}

/// A list of labels in the order of the labels.
fn sorted(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(mut items) => {
            items.sort_by_key(ToString::to_string);
            serde_json::Value::Array(items)
        }
        other => other,
    }
}

/// The name of the kind of record the values of a change are labelled as: the field in the change
/// (`reported-by`), or `own-field` for a field somebody made, which is written by its ID.
const OWN_FIELD: &str = "own-field";

fn dump_event(labels: &mut Labels, event: &DomainEvent) -> DumpEvent {
    DumpEvent {
        time: event.time.to_string(),
        actor: labels.actor(&event.actor),
        channel: event.channel.code(),
        entity: labels.entity(&event.entity),
        changes: event
            .changes
            .iter()
            .map(|change| {
                // An own field is named by an ID in its events, which is random by design.
                let (field, kind) = match id_bytes(&change.field) {
                    Some(id) => (labels.label(OWN_FIELD, id), OWN_FIELD),
                    None => (change.field.clone(), change.field.as_str()),
                };
                // The people an issue is assigned to are a set: the order they are listed in comes
                // from their random IDs, so the dump lists them by label.
                let set = change.field == Field::AssignedTo.code();
                let mut value = |value: &Option<serde_json::Value>| {
                    value.clone().map(|value| {
                        let value = labelled(labels, kind, value);
                        if set { sorted(value) } else { value }
                    })
                };
                DumpChange {
                    field,
                    before: value(&change.before),
                    after: value(&change.after),
                }
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
