//! One issue of the large dataset with everything that belongs to it: its events in the order they
//! happened, its comments, followers, tags, labels, parent, relation and file. Everything about
//! issue number `n` comes from the seed and `n` alone, so a run makes the same dataset whatever the
//! size of the batches it is written in.

use bb_app::dataset_events as events;
use bb_core::comment::Comment;
use bb_core::event::{Actor, Channel, DomainEvent, EntityRef, RecordKind};
use bb_core::files::{Attachment, PreviewStatus};
use bb_core::follow::{FollowReason, Follower, who_starts_following};
use bb_core::id::{AttachmentId, CommentId, EntityId, EventId, IssueId, RelationId, UserId};
use bb_core::issue::{AssignedPerson, Issue, StatusGroup};
use bb_core::own_fields::FieldType;
use bb_core::own_fields_value::{Cell, FieldValue};
use bb_core::readable_id::{IssueCode, code_length, longest_code};
use bb_core::relation::{IssueParent, Relation, SUB_ISSUES_MAX};
use bb_core::tags::{IssueLabel, IssueTag};
use bb_core::text::CodeDraws;
use bb_core::version::FIRST_VERSION;
use jiff::Timestamp;
use serde_json::json;

use crate::error::SampleError;
use crate::issues_plan::FieldPick;
use crate::issues_text::{describe, field_pick, priority};
use crate::large_ids::{Draws, ordered, rng_for};
use crate::large_org::{Org, Place};
use crate::large_state::{Bank, Counters};
use crate::random::SeededRandom;

/// The kinds of random numbers an issue draws, each from a stream of its own.
const ISSUE: u64 = 1;
const EVENT_ID: u64 = 2;

/// Seconds an event may come after the thing before it, at most: a status change comes within a
/// month, a comment within two weeks.
const STATUS_WITHIN: i64 = 30 * 24 * 3600;
const COMMENT_WITHIN: i64 = 14 * 24 * 3600;
/// The shortest wait before anything happens to a new issue, in seconds.
const AT_LEAST: i64 = 120;
/// How many issues of a project a new relation may point at: the latest ones.
const RECENT: usize = 300;
/// How many goals of a project can be given sub-issues.
const GOALS: usize = 20;
/// How many labels an issue carries at most, and how many tags.
const LABELS_AT_MOST: usize = 2;
const TAGS_AT_MOST: usize = 3;

/// Everything written for one issue.
pub(crate) struct Bundle {
    pub(crate) issue: Issue,
    pub(crate) assigned: Vec<AssignedPerson>,
    pub(crate) values: Vec<(bb_core::id::FieldId, Vec<Cell>)>,
    pub(crate) followers: Vec<Follower>,
    pub(crate) comments: Vec<Comment>,
    pub(crate) tags: Vec<IssueTag>,
    pub(crate) labels: Vec<IssueLabel>,
    pub(crate) parent: Option<IssueParent>,
    pub(crate) relation: Option<Relation>,
    pub(crate) attachments: Vec<Attachment>,
    pub(crate) events: Vec<DomainEvent>,
}

/// What happens to an issue after it was made and counts its version up.
enum Step {
    Status(usize),
    Tag(usize),
    Label(usize),
    Parent,
}

/// Makes the issues one after the other.
pub(crate) struct Maker<'a> {
    pub(crate) bank: &'a Bank,
    pub(crate) org: &'a Org,
    pub(crate) seed: u64,
    pub(crate) total: u32,
    pub(crate) begin: i64,
    pub(crate) end: i64,
    pub(crate) counters: Counters,
    pub(crate) cumulative: Vec<u64>,
    pub(crate) type_cumulative: Vec<u32>,
}

fn event(
    random: &mut SeededRandom,
    time: Timestamp,
    actor: UserId,
    kind: RecordKind,
    id: impl Into<EntityId>,
    changes: Vec<bb_core::event::FieldChange>,
) -> Result<DomainEvent, SampleError> {
    Ok(DomainEvent {
        id: ordered(EventId::from_parts, time, random)?,
        time,
        actor: Actor::Person(actor),
        channel: Channel::Ui,
        entity: EntityRef {
            kind: kind.into(),
            id: id.into(),
        },
        changes,
        reason: None,
    })
}

fn moment(seconds: i64) -> Timestamp {
    Timestamp::from_second(seconds).unwrap_or(Timestamp::UNIX_EPOCH)
}

impl Maker<'_> {
    fn pick_place(&self, random: &mut SeededRandom) -> usize {
        let total = self.cumulative.last().copied().unwrap_or(1).max(1);
        let roll =
            u64::try_from(random.below(usize::try_from(total).unwrap_or(usize::MAX))).unwrap_or(0);
        self.cumulative.partition_point(|upto| *upto <= roll)
    }

    fn pick_type(&self, random: &mut SeededRandom) -> usize {
        let roll = u32::try_from(random.below(100)).unwrap_or(0);
        self.type_cumulative.partition_point(|upto| *upto <= roll)
    }

    fn team(&self, place: &Place, random: &mut SeededRandom) -> UserId {
        let everywhere = &self.org.everywhere;
        let from_team = !place.team.is_empty() && (everywhere.is_empty() || random.chance(60));
        let pool = if from_team { &place.team } else { everywhere };
        let at = pool.get(random.below(pool.len())).copied().unwrap_or(0);
        self.org.folk[at].id
    }

    /// Who reported the issue: mostly the team; sometimes a Contributor or a Guest who holds a
    /// role in the project.
    fn reporter(&self, place: &Place, random: &mut SeededRandom) -> (UserId, bool) {
        if !place.outsiders.is_empty() && random.chance(12) {
            let at = place.outsiders[random.below(place.outsiders.len())];
            return (self.org.folk[at].id, true);
        }
        (self.team(place, random), false)
    }

    fn title_and_text(
        &self,
        place: &Place,
        type_at: usize,
        random: &mut SeededRandom,
    ) -> (String, String) {
        let bank = self.bank;
        if place.german {
            let title = random
                .pick(&bank.german.titles)
                .cloned()
                .unwrap_or_default();
            let text = random
                .pick(&bank.german.descriptions)
                .cloned()
                .unwrap_or_default();
            return (title, format!("{text}\n"));
        }
        let words = &bank.words;
        let (templates, bug) = match type_at {
            0 => (&words.bug, true),
            1 => (&words.task, false),
            2 => (&words.feature, false),
            _ => (&words.goal, false),
        };
        let template = random.pick(&templates.titles).cloned().unwrap_or_default();
        let title = words.fill(&template, &place.area, random);
        (title, describe(words, templates, bug, &place.area, random))
    }

    fn comment_text(&self, place: &Place, random: &mut SeededRandom) -> String {
        let bank = self.bank;
        let list = if place.german {
            &bank.german.comments
        } else {
            &bank.english
        };
        random.pick(list).cloned().unwrap_or_default()
    }

    fn code(
        &mut self,
        place_at: usize,
        random_codes: bool,
        random: &mut SeededRandom,
    ) -> IssueCode {
        if !random_codes {
            self.counters.last[place_at] += 1;
            return IssueCode::counting(self.counters.last[place_at]);
        }
        let mut source = Draws(random);
        for symbols in CodeDraws::new(&mut source, code_length(5), longest_code()) {
            if let Some(code) = IssueCode::random(&symbols) {
                if self.counters.codes[place_at].insert(symbols) {
                    return code;
                }
            }
        }
        // Not reached: the draws run to the longest code. Fall back to a counting one.
        self.counters.last[place_at] += 1;
        IssueCode::counting(self.counters.last[place_at])
    }

    /// Issue number `index`, with everything that belongs to it.
    pub(crate) fn build(&mut self, index: u32) -> Result<Bundle, SampleError> {
        let bank = self.bank;
        let plan = &bank.plan;
        let mut random = rng_for(self.seed, ISSUE, u64::from(index));
        let mut ids = rng_for(self.seed, EVENT_ID, u64::from(index));
        let place_at = self.pick_place(&mut random);
        let place = &self.org.places[place_at];
        let type_at = self.pick_type(&mut random);
        let span = (self.end - self.begin).max(1);
        let created = self.begin + span * i64::from(index) / i64::from(self.total.max(1));
        let created_at = moment(created);
        let (reporter, outsider) = self.reporter(place, &mut random);
        let (title, description) = self.title_and_text(place, type_at, &mut random);
        let is_bug = type_at == 0;
        let ladder = &bank.ladders[place_at][type_at];

        let assigned_to: Vec<UserId> =
            if !outsider && (ladder.needs_person || random.chance(plan.assigned_percent)) {
                let mut people = vec![self.team(place, &mut random)];
                if random.chance(10) {
                    people.push(self.team(place, &mut random));
                    people.dedup();
                }
                people
            } else {
                Vec::new()
            };

        // The road through the workflow, up to where the issue is by now: the older the issue,
        // the likelier it is done.
        let age = f64::from(index) / f64::from(self.total.max(1));
        let progress = {
            let last = ladder.steps.len().saturating_sub(1);
            let done = f64::from(plan.done_percent_oldest) * (1.0 - age)
                + f64::from(plan.done_percent_newest) * age;
            let roll = f64::from(u32::try_from(random.below(100)).unwrap_or(0));
            if outsider || last == 0 {
                0
            } else if roll < done {
                last
            } else {
                random.below(last)
            }
        };

        let id = ordered(IssueId::from_parts, created_at, &mut ids)?;
        let code = self.code(place_at, place.random_codes, &mut random);
        let (first_status, _) = ladder
            .steps
            .first()
            .cloned()
            .ok_or_else(|| SampleError::NotMade(place.short_name.clone()))?;
        let mut issue = Issue {
            id,
            project: place.id,
            issue_type: bank.types[type_at],
            code,
            title,
            description,
            status: first_status,
            priority: priority(is_bug, &mut random),
            reported_by: reporter,
            created_at,
            updated_at: created_at,
            closed_at: None,
            deleted_at: None,
            version: FIRST_VERSION,
        };
        let assigned: Vec<AssignedPerson> = assigned_to
            .iter()
            .map(|person| AssignedPerson {
                issue: id,
                person: *person,
                added_at: created_at,
            })
            .collect();

        // The values of the own fields.
        let family = &bank.families[place_at];
        let type_name = bank
            .names
            .types
            .get(type_at)
            .map_or("", |share| share.name.as_str());
        let mut own_changes = Vec::new();
        let mut values = Vec::new();
        let mut set = |field: bb_core::id::FieldId, kind: FieldType, value: &FieldValue| {
            own_changes.push(events::own_value_set(field, kind, value));
            values.push((field, value.to_cells()));
        };
        for (field_plan, made) in &bank.fields {
            match field_pick(field_plan, type_name, family, &bank.words, &mut random) {
                Some(FieldPick::Choices(places)) => {
                    let chosen: Vec<[u8; 16]> = places
                        .iter()
                        .filter_map(|each| made.choices.get(*each))
                        .map(|choice| choice.to_bytes())
                        .collect();
                    set(made.id, made.kind, &FieldValue::Ids(chosen));
                }
                Some(FieldPick::Text(text)) => set(made.id, made.kind, &FieldValue::Text(text)),
                None => {}
            }
        }
        if !outsider && random.chance(25) {
            let cost = format!("CC-{}", 1000 + random.below(900));
            set(bank.cost_center, FieldType::Text, &FieldValue::Text(cost));
        }

        // What happens after: status changes, tags, labels, the parent.
        let mut steps: Vec<(i64, Step)> = Vec::new();
        let mut at = created + AT_LEAST;
        for step in 1..=progress {
            at += AT_LEAST
                + i64::try_from(
                    random.below(usize::try_from(STATUS_WITHIN).unwrap_or(1) / progress.max(1)),
                )
                .unwrap_or(0);
            steps.push((at, Step::Status(step)));
        }
        let tags: Vec<usize> = if random.chance(plan.tag_percent) && !bank.tags.is_empty() {
            let mut found = vec![random.below(bank.tags.len())];
            for _ in 0..random.below(TAGS_AT_MOST) {
                let more = random.below(bank.tags.len());
                if !found.contains(&more) {
                    found.push(more);
                }
            }
            found
        } else {
            Vec::new()
        };
        for tag in &tags {
            let when = created + AT_LEAST + i64::try_from(random.below(86_400)).unwrap_or(0);
            steps.push((when, Step::Tag(*tag)));
        }
        let mut labels: Vec<usize> = Vec::new();
        if random.chance(plan.label_percent) && !bank.labels.is_empty() {
            for _ in 0..=random.below(LABELS_AT_MOST) {
                let pick = random.below(bank.labels.len());
                let group = bank.labels[pick].group;
                let clash = labels.iter().any(|each| {
                    *each == pick || (group.is_some() && bank.labels[*each].group == group)
                });
                if !clash {
                    labels.push(pick);
                }
            }
        }
        for label in &labels {
            let when = created + AT_LEAST + i64::try_from(random.below(86_400)).unwrap_or(0);
            steps.push((when, Step::Label(*label)));
        }
        let goal = bank
            .names
            .types
            .iter()
            .position(|share| share.name == "Goal");
        let parent = if !outsider && Some(type_at) != goal && random.chance(plan.sub_issue_percent)
        {
            let goals = &mut self.counters.goals[place_at];
            let open: Vec<usize> = goals
                .iter()
                .enumerate()
                .filter(|(_, (_, children))| *children < SUB_ISSUES_MAX)
                .map(|(at_goal, _)| at_goal)
                .collect();
            open.get(random.below(open.len().max(1)))
                .copied()
                .map(|at_goal| {
                    goals[at_goal].1 += 1;
                    goals[at_goal].0
                })
        } else {
            None
        };
        if parent.is_some() {
            steps.push((created + AT_LEAST + 1, Step::Parent));
        }
        if Some(type_at) == goal {
            let goals = &mut self.counters.goals[place_at];
            goals.push((id, 0));
            if goals.len() > GOALS {
                goals.remove(0);
            }
        }
        steps.sort_by_key(|(when, _)| *when);

        let mut events = Vec::new();
        events.push(event(
            &mut ids,
            created_at,
            reporter,
            RecordKind::Issue,
            id,
            events::issue_created(&issue, &assigned_to, own_changes),
        )?);
        let mut tag_names: Vec<String> = Vec::new();
        let mut label_set: Vec<bb_core::tags::Label> = Vec::new();
        let mut tag_links = Vec::new();
        let mut label_links = Vec::new();
        let mut parent_link = None;
        let doer = assigned_to.first().copied().unwrap_or(reporter);
        for (when, step) in steps {
            let time = moment(when.min(self.end));
            let before = issue.version;
            let (actor, changes) = match step {
                Step::Status(step) => {
                    let (key, group) = ladder.steps[step].clone();
                    let changes = events::status_changed(&issue.status, &key, before);
                    issue.status = key;
                    if group == StatusGroup::Done {
                        issue.closed_at = Some(time);
                    }
                    (doer, changes)
                }
                Step::Tag(tag) => {
                    let name = bank.tags[tag].name.clone();
                    let mut after = tag_names.clone();
                    after.push(name);
                    let changes = events::tag_put_on(&tag_names, &after, before);
                    tag_names = after;
                    tag_links.push(IssueTag {
                        issue: id,
                        tag: bank.tags[tag].id,
                        added_by: reporter,
                        added_at: time,
                    });
                    (reporter, changes)
                }
                Step::Label(label) => {
                    let mut after = label_set.clone();
                    after.push(bank.labels[label].clone());
                    let changes = events::label_put_on(&label_set, &after, before);
                    label_set = after;
                    label_links.push(IssueLabel {
                        issue: id,
                        label: bank.labels[label].id,
                        added_by: reporter,
                        added_at: time,
                    });
                    (reporter, changes)
                }
                Step::Parent => {
                    let above = parent.ok_or_else(|| SampleError::NotMade("parent".to_owned()))?;
                    parent_link = Some(IssueParent {
                        child: id,
                        parent: above,
                        set_by: reporter,
                        set_at: time,
                    });
                    (reporter, events::parent_changed(None, Some(above), before))
                }
            };
            issue.version = before + 1;
            issue.updated_at = time;
            events.push(event(
                &mut ids,
                time,
                actor,
                RecordKind::Issue,
                id,
                changes,
            )?);
        }

        // Followers: whoever reported or was assigned, whoever commented, and a few more.
        let mut followers: Vec<Follower> = who_starts_following(Some(reporter), &assigned_to, None)
            .into_iter()
            .map(|(person, reason)| Follower {
                issue: id,
                person,
                reason,
                since: created_at,
            })
            .collect();
        if !outsider {
            for _ in 0..random.below(usize::try_from(plan.followers_at_most).unwrap_or(0) + 1) {
                let person = self.team(place, &mut random);
                if followers.iter().all(|each| each.person != person) {
                    followers.push(Follower {
                        issue: id,
                        person,
                        reason: FollowReason::Chose,
                        since: created_at,
                    });
                }
            }
        }

        // Comments.
        let mut comments = Vec::new();
        let per_issue = usize::try_from(plan.comments_per_100).unwrap_or(0) / 100;
        for _ in 0..random.below(per_issue * 2 + 1) {
            let author = if outsider {
                reporter
            } else if random.chance(50) {
                doer
            } else {
                self.team(place, &mut random)
            };
            let when = created
                + AT_LEAST
                + i64::try_from(random.below(usize::try_from(COMMENT_WITHIN).unwrap_or(1)))
                    .unwrap_or(0);
            let time = moment(when.min(self.end));
            let comment = Comment {
                id: ordered(CommentId::from_parts, time, &mut ids)?,
                issue: id,
                author,
                text: self.comment_text(place, &mut random),
                created_at: time,
                edited_at: None,
                deleted_at: None,
                version: FIRST_VERSION,
            };
            events.push(event(
                &mut ids,
                time,
                author,
                RecordKind::Comment,
                comment.id,
                events::comment_written(id, &comment.text),
            )?);
            for (person, reason) in who_starts_following(None, &[], Some(author)) {
                if followers.iter().all(|each| each.person != person) {
                    followers.push(Follower {
                        issue: id,
                        person,
                        reason,
                        since: time,
                    });
                }
            }
            comments.push(comment);
        }

        // A relation to a recent issue of the project, and a file.
        let relation = if !self.counters.recent[place_at].is_empty()
            && !bank.kinds.is_empty()
            && random.chance(plan.relations_per_100)
        {
            let recent = &self.counters.recent[place_at];
            let other = recent[random.below(recent.len())];
            let time = moment((created + AT_LEAST * 2).min(self.end));
            let relation = Relation {
                id: ordered(RelationId::from_parts, time, &mut ids)?,
                kind: bank.kinds[random.below(bank.kinds.len())],
                from: id,
                to: other,
                added_by: reporter,
                added_at: time,
            };
            events.push(event(
                &mut ids,
                time,
                reporter,
                RecordKind::Relation,
                relation.id,
                events::relation_added(&relation),
            )?);
            Some(relation)
        } else {
            None
        };
        let mut attachments = Vec::new();
        if random.chance(plan.file_percent) {
            let which = random.below(bank.files.len());
            let file = &bank.files[which];
            let time = moment((created + AT_LEAST * 3).min(self.end));
            let attachment = Attachment {
                id: ordered(AttachmentId::from_parts, time, &mut ids)?,
                issue: id,
                comment: None,
                file: file.hash,
                name: format!("attachment-{index}.{}", extension(file.media_type.as_str())),
                added_by: reporter,
                added_at: time,
                deleted_at: None,
                preview: PreviewStatus::Without,
                version: FIRST_VERSION,
            };
            events.push(event(
                &mut ids,
                time,
                reporter,
                RecordKind::Attachment,
                attachment.id,
                events::attachment_added(&attachment, file),
            )?);
            attachments.push(attachment);
        }

        let recent = &mut self.counters.recent[place_at];
        recent.push_back(id);
        if recent.len() > RECENT {
            recent.pop_front();
        }
        let _ = json!(null);
        Ok(Bundle {
            issue,
            assigned,
            values,
            followers,
            comments,
            tags: tag_links,
            labels: label_links,
            parent: parent_link,
            relation,
            attachments,
            events,
        })
    }
}

fn extension(media_type: &str) -> &'static str {
    match media_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        _ => "pdf",
    }
}
