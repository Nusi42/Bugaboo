//! One issue of the large dataset with everything that belongs to it: its events in the order they
//! happened, its comments, followers, tags, labels, parent, relation and file. Everything about
//! issue number `n` comes from the seed and `n` alone, so a run makes the same dataset whatever the
//! size of the batches it is written in.

use bb_app::event_members as events;
use bb_core::comment::Comment;
use bb_core::event::{Actor, Channel, DomainEvent, EntityRef, FieldChange, RecordKind};
use bb_core::files::{Attachment, PreviewStatus};
use bb_core::follow::{FollowReason, Follower, who_starts_following};
use bb_core::id::{
    AttachmentId, CommentId, EntityId, EventId, FieldId, IssueId, RelationId, UserId,
};
use bb_core::issue::{AssignedPerson, Issue, StatusGroup};
use bb_core::own_fields::FieldType;
use bb_core::own_fields_value::{Cell, FieldValue};
use bb_core::readable_id::{IssueCode, code_length, longest_code};
use bb_core::relation::{IssueParent, Relation, SUB_ISSUES_MAX};
use bb_core::tags::{IssueLabel, IssueTag, Label};
use bb_core::text::CodeDraws;
use bb_core::version::FIRST_VERSION;
use jiff::Timestamp;

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
    pub(crate) values: Vec<(FieldId, Vec<Cell>)>,
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

pub(crate) fn event(
    random: &mut SeededRandom,
    time: Timestamp,
    actor: UserId,
    kind: RecordKind,
    id: impl Into<EntityId>,
    changes: Vec<FieldChange>,
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
            if let Some(code) = IssueCode::random(&symbols)
                && self.counters.codes[place_at].insert(symbols)
            {
                return code;
            }
        }
        // Not reached: the draws run to the longest code. Fall back to a counting one.
        self.counters.last[place_at] += 1;
        IssueCode::counting(self.counters.last[place_at])
    }

    /// Issue number `index`, with everything that belongs to it.
    pub(crate) fn build(&mut self, index: u32) -> Result<Bundle, SampleError> {
        let mut facts = self.facts(index)?;
        let (assigned_to, progress) = self.assignment_and_progress(&mut facts);
        let place = &self.org.places[facts.place_at];
        let mut issue = self.new_issue(&mut facts, place, &assigned_to)?;
        let (own_changes, values) = self.own_values(&mut facts);
        let (steps, parent) = self.after_steps(&mut facts, progress);
        let mut events = vec![event(
            &mut facts.ids,
            facts.created_at,
            facts.reporter,
            RecordKind::Issue,
            facts.id,
            events::issue_created(&issue, &assigned_to, own_changes),
        )?];
        let doer = assigned_to.first().copied().unwrap_or(facts.reporter);
        let done = self.apply_steps(&mut facts, &mut issue, steps, parent, doer)?;
        events.extend(done.events);
        let talk = self.talk(&mut facts, place, &assigned_to, doer)?;
        events.extend(talk.events);
        let relation = self.relation(&mut facts, &mut events)?;
        let attachments = self.attachment(&mut facts, &mut events)?;
        let recent = &mut self.counters.recent[facts.place_at];
        recent.push_back(facts.id);
        if recent.len() > RECENT {
            recent.pop_front();
        }
        let assigned = assigned_to
            .iter()
            .map(|person| AssignedPerson {
                issue: facts.id,
                person: *person,
                added_at: facts.created_at,
            })
            .collect();
        Ok(Bundle {
            issue,
            assigned,
            values,
            followers: talk.followers,
            comments: talk.comments,
            tags: done.tags,
            labels: done.labels,
            parent: done.parent,
            relation,
            attachments,
            events,
        })
    }

    /// What is decided first about an issue: where, of which type, when, by whom.
    fn facts(&self, index: u32) -> Result<Facts, SampleError> {
        let mut random = rng_for(self.seed, ISSUE, u64::from(index));
        let mut ids = rng_for(self.seed, EVENT_ID, u64::from(index));
        let place_at = self.pick_place(&mut random);
        let type_at = self.pick_type(&mut random);
        let span = (self.end - self.begin).max(1);
        let created = self.begin + span * i64::from(index) / i64::from(self.total.max(1));
        let created_at = moment(created);
        let place = &self.org.places[place_at];
        let (reporter, outsider) = self.reporter(place, &mut random);
        let id = ordered(IssueId::from_parts, created_at, &mut ids)?;
        Ok(Facts {
            index,
            place_at,
            type_at,
            created,
            created_at,
            reporter,
            outsider,
            id,
            random,
            ids,
        })
    }

    /// Who the issue is assigned to, and how far along the workflow it is: the older the issue,
    /// the likelier it is done.
    fn assignment_and_progress(&self, facts: &mut Facts) -> (Vec<UserId>, usize) {
        let plan = &self.bank.plan;
        let place = &self.org.places[facts.place_at];
        let ladder = &self.bank.ladders[facts.place_at][facts.type_at];
        // An issue that a Contributor or a Guest reported is always assigned to the team and goes
        // on like the others, so the team's changes and comments tell the reporter, which the
        // rules of what they are told need to look at.
        let assigned = if facts.outsider
            || ladder.needs_person
            || facts.random.chance(plan.assigned_percent)
        {
            let mut people = vec![self.team(place, &mut facts.random)];
            if facts.random.chance(10) {
                people.push(self.team(place, &mut facts.random));
                people.dedup();
            }
            people
        } else {
            Vec::new()
        };
        let age = f64::from(facts.index) / f64::from(self.total.max(1));
        let last = ladder.steps.len().saturating_sub(1);
        let done = f64::from(plan.done_percent_oldest) * (1.0 - age)
            + f64::from(plan.done_percent_newest) * age;
        let roll = f64::from(u32::try_from(facts.random.below(100)).unwrap_or(0));
        let progress = if last == 0 {
            0
        } else if roll < done {
            last
        } else {
            facts.random.below(last)
        };
        (assigned, progress)
    }

    fn new_issue(
        &mut self,
        facts: &mut Facts,
        place: &Place,
        assigned_to: &[UserId],
    ) -> Result<Issue, SampleError> {
        let _ = assigned_to;
        let bank = self.bank;
        let ladder = &bank.ladders[facts.place_at][facts.type_at];
        let (title, description) = self.title_and_text(place, facts.type_at, &mut facts.random);
        let code = self.code(facts.place_at, place.random_codes, &mut facts.random);
        let (first_status, _) = ladder
            .steps
            .first()
            .cloned()
            .ok_or_else(|| SampleError::NotMade(place.short_name.clone()))?;
        Ok(Issue {
            id: facts.id,
            project: place.id,
            issue_type: bank.types[facts.type_at],
            code,
            title,
            description,
            status: first_status,
            priority: priority(facts.type_at == 0, &mut facts.random),
            reported_by: facts.reporter,
            created_at: facts.created_at,
            updated_at: facts.created_at,
            closed_at: None,
            deleted_at: None,
            version: FIRST_VERSION,
        })
    }

    /// The values of the own fields, as event members and as stored cells.
    fn own_values(&self, facts: &mut Facts) -> (Vec<FieldChange>, Vec<(FieldId, Vec<Cell>)>) {
        let bank = self.bank;
        let family = &bank.families[facts.place_at];
        let type_name = bank
            .names
            .types
            .get(facts.type_at)
            .map_or("", |share| share.name.as_str());
        let mut changes = Vec::new();
        let mut values = Vec::new();
        let mut set = |field: FieldId, kind: FieldType, value: &FieldValue| {
            changes.push(events::own_value_set(field, kind, value));
            values.push((field, value.to_cells()));
        };
        for (field_plan, made) in &bank.fields {
            match field_pick(
                field_plan,
                type_name,
                family,
                &bank.words,
                &mut facts.random,
            ) {
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
        if !facts.outsider && facts.random.chance(25) {
            let cost = format!("CC-{}", 1000 + facts.random.below(900));
            set(bank.cost_center, FieldType::Text, &FieldValue::Text(cost));
        }
        (changes, values)
    }

    fn tag_picks(&self, facts: &mut Facts) -> Vec<usize> {
        let bank = self.bank;
        if bank.tags.is_empty() || !facts.random.chance(bank.plan.tag_percent) {
            return Vec::new();
        }
        let mut found = vec![facts.random.below(bank.tags.len())];
        for _ in 0..facts.random.below(TAGS_AT_MOST) {
            let more = facts.random.below(bank.tags.len());
            if !found.contains(&more) {
                found.push(more);
            }
        }
        found
    }

    fn label_picks(&self, facts: &mut Facts) -> Vec<usize> {
        let bank = self.bank;
        let mut labels: Vec<usize> = Vec::new();
        if bank.labels.is_empty() || !facts.random.chance(bank.plan.label_percent) {
            return labels;
        }
        for _ in 0..=facts.random.below(LABELS_AT_MOST) {
            let pick = facts.random.below(bank.labels.len());
            let group = bank.labels[pick].group;
            let clash = labels.iter().any(|each| {
                *each == pick || (group.is_some() && bank.labels[*each].group == group)
            });
            if !clash {
                labels.push(pick);
            }
        }
        labels
    }

    /// The goal this issue is a sub-issue of, and the goals of the project kept up to date.
    fn parent_pick(&mut self, facts: &mut Facts) -> Option<IssueId> {
        let bank = self.bank;
        let goal = bank
            .names
            .types
            .iter()
            .position(|share| share.name == "Goal");
        let goals = &mut self.counters.goals[facts.place_at];
        let wanted = !facts.outsider
            && Some(facts.type_at) != goal
            && facts.random.chance(bank.plan.sub_issue_percent);
        let found = if wanted {
            let open: Vec<usize> = goals
                .iter()
                .enumerate()
                .filter(|(_, (_, children))| *children < SUB_ISSUES_MAX)
                .map(|(at_goal, _)| at_goal)
                .collect();
            open.get(facts.random.below(open.len().max(1)))
                .copied()
                .map(|at_goal| {
                    goals[at_goal].1 += 1;
                    goals[at_goal].0
                })
        } else {
            None
        };
        if Some(facts.type_at) == goal {
            goals.push((facts.id, 0));
            if goals.len() > GOALS {
                goals.remove(0);
            }
        }
        found
    }

    /// What happens to the issue after it was made, in the order it happens.
    fn after_steps(
        &mut self,
        facts: &mut Facts,
        progress: usize,
    ) -> (Vec<(i64, Step)>, Option<IssueId>) {
        let mut steps: Vec<(i64, Step)> = Vec::new();
        let mut at = facts.created + AT_LEAST;
        let within = usize::try_from(STATUS_WITHIN).unwrap_or(1) / progress.max(1);
        for step in 1..=progress {
            at += AT_LEAST + i64::try_from(facts.random.below(within)).unwrap_or(0);
            steps.push((at, Step::Status(step)));
        }
        let soon = |facts: &mut Facts| {
            facts.created + AT_LEAST + i64::try_from(facts.random.below(86_400)).unwrap_or(0)
        };
        for tag in self.tag_picks(facts) {
            steps.push((soon(facts), Step::Tag(tag)));
        }
        for label in self.label_picks(facts) {
            steps.push((soon(facts), Step::Label(label)));
        }
        let parent = self.parent_pick(facts);
        if parent.is_some() {
            steps.push((facts.created + AT_LEAST + 1, Step::Parent));
        }
        steps.sort_by_key(|(when, _)| *when);
        (steps, parent)
    }

    /// Carries the steps out: each counts the version of the issue up and has its own event.
    fn apply_steps(
        &self,
        facts: &mut Facts,
        issue: &mut Issue,
        steps: Vec<(i64, Step)>,
        parent: Option<IssueId>,
        doer: UserId,
    ) -> Result<Done, SampleError> {
        let bank = self.bank;
        let ladder = &bank.ladders[facts.place_at][facts.type_at];
        let mut done = Done::default();
        let mut tag_names: Vec<String> = Vec::new();
        let mut label_set: Vec<Label> = Vec::new();
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
                    let mut after = tag_names.clone();
                    after.push(bank.tags[tag].name.clone());
                    let changes = events::tag_put_on(&tag_names, &after, before);
                    tag_names = after;
                    done.tags.push(IssueTag {
                        issue: facts.id,
                        tag: bank.tags[tag].id,
                        added_by: facts.reporter,
                        added_at: time,
                    });
                    (facts.reporter, changes)
                }
                Step::Label(label) => {
                    let mut after = label_set.clone();
                    after.push(bank.labels[label].clone());
                    let changes = events::label_put_on(&label_set, &after, before);
                    label_set = after;
                    done.labels.push(IssueLabel {
                        issue: facts.id,
                        label: bank.labels[label].id,
                        added_by: facts.reporter,
                        added_at: time,
                    });
                    (facts.reporter, changes)
                }
                Step::Parent => {
                    let above = parent.ok_or_else(|| SampleError::NotMade("parent".to_owned()))?;
                    done.parent = Some(IssueParent {
                        child: facts.id,
                        parent: above,
                        set_by: facts.reporter,
                        set_at: time,
                    });
                    (
                        facts.reporter,
                        events::parent_changed(None, Some(above), before),
                    )
                }
            };
            issue.version = before + 1;
            issue.updated_at = time;
            done.events.push(event(
                &mut facts.ids,
                time,
                actor,
                RecordKind::Issue,
                facts.id,
                changes,
            )?);
        }
        Ok(done)
    }

    /// Followers (whoever reported, was assigned or commented, and a few more) and the comments
    /// with their events.
    fn talk(
        &self,
        facts: &mut Facts,
        place: &Place,
        assigned_to: &[UserId],
        doer: UserId,
    ) -> Result<Talk, SampleError> {
        let plan = &self.bank.plan;
        let id = facts.id;
        let follower = |person, reason, since| Follower {
            issue: id,
            person,
            reason,
            since,
        };
        let mut followers: Vec<Follower> =
            who_starts_following(Some(facts.reporter), assigned_to, None)
                .into_iter()
                .map(|(person, reason)| follower(person, reason, facts.created_at))
                .collect();
        if !facts.outsider {
            let more = usize::try_from(plan.followers_at_most).unwrap_or(0) + 1;
            for _ in 0..facts.random.below(more) {
                let person = self.team(place, &mut facts.random);
                if followers.iter().all(|each| each.person != person) {
                    followers.push(follower(person, FollowReason::Chose, facts.created_at));
                }
            }
        }
        let mut comments = Vec::new();
        let mut events_of = Vec::new();
        let per_issue = usize::try_from(plan.comments_per_100).unwrap_or(0) / 100;
        for _ in 0..facts.random.below(per_issue * 2 + 1) {
            let author = if facts.outsider && facts.random.chance(50) {
                facts.reporter
            } else if facts.random.chance(50) {
                doer
            } else {
                self.team(place, &mut facts.random)
            };
            let within = usize::try_from(COMMENT_WITHIN).unwrap_or(1);
            let when =
                facts.created + AT_LEAST + i64::try_from(facts.random.below(within)).unwrap_or(0);
            let time = moment(when.min(self.end));
            let comment = Comment {
                id: ordered(CommentId::from_parts, time, &mut facts.ids)?,
                issue: id,
                author,
                text: self.comment_text(place, &mut facts.random),
                created_at: time,
                edited_at: None,
                deleted_at: None,
                version: FIRST_VERSION,
            };
            events_of.push(event(
                &mut facts.ids,
                time,
                author,
                RecordKind::Comment,
                comment.id,
                events::comment_written(id, &comment.text),
            )?);
            for (person, reason) in who_starts_following(None, &[], Some(author)) {
                if followers.iter().all(|each| each.person != person) {
                    followers.push(follower(person, reason, time));
                }
            }
            comments.push(comment);
        }
        Ok(Talk {
            followers,
            comments,
            events: events_of,
        })
    }

    /// A relation to a recent issue of the project.
    fn relation(
        &self,
        facts: &mut Facts,
        events_of: &mut Vec<DomainEvent>,
    ) -> Result<Option<Relation>, SampleError> {
        let bank = self.bank;
        let recent = &self.counters.recent[facts.place_at];
        if recent.is_empty()
            || bank.kinds.is_empty()
            || !facts.random.chance(bank.plan.relations_per_100)
        {
            return Ok(None);
        }
        let other = recent[facts.random.below(recent.len())];
        let time = moment((facts.created + AT_LEAST * 2).min(self.end));
        let relation = Relation {
            id: ordered(RelationId::from_parts, time, &mut facts.ids)?,
            kind: bank.kinds[facts.random.below(bank.kinds.len())],
            from: facts.id,
            to: other,
            added_by: facts.reporter,
            added_at: time,
        };
        events_of.push(event(
            &mut facts.ids,
            time,
            facts.reporter,
            RecordKind::Relation,
            relation.id,
            events::relation_added(&relation),
        )?);
        Ok(Some(relation))
    }

    /// A file record on the issue.
    fn attachment(
        &self,
        facts: &mut Facts,
        events_of: &mut Vec<DomainEvent>,
    ) -> Result<Vec<Attachment>, SampleError> {
        let bank = self.bank;
        if !facts.random.chance(bank.plan.file_percent) {
            return Ok(Vec::new());
        }
        let file = &bank.files[facts.random.below(bank.files.len())];
        let time = moment((facts.created + AT_LEAST * 3).min(self.end));
        let attachment = Attachment {
            id: ordered(AttachmentId::from_parts, time, &mut facts.ids)?,
            issue: facts.id,
            comment: None,
            file: file.hash,
            name: format!(
                "attachment-{}.{}",
                facts.index,
                extension(file.media_type.as_str())
            ),
            added_by: facts.reporter,
            added_at: time,
            deleted_at: None,
            preview: PreviewStatus::Without,
            version: FIRST_VERSION,
        };
        events_of.push(event(
            &mut facts.ids,
            time,
            facts.reporter,
            RecordKind::Attachment,
            attachment.id,
            events::attachment_added(&attachment, file),
        )?);
        Ok(vec![attachment])
    }
}

/// What is decided first about an issue, and the random streams it draws from.
struct Facts {
    index: u32,
    place_at: usize,
    type_at: usize,
    created: i64,
    created_at: Timestamp,
    reporter: UserId,
    outsider: bool,
    id: IssueId,
    random: SeededRandom,
    ids: SeededRandom,
}

/// The followers and comments of an issue, with the events of the comments.
struct Talk {
    followers: Vec<Follower>,
    comments: Vec<Comment>,
    events: Vec<DomainEvent>,
}

/// What the steps after the making of an issue leave behind.
#[derive(Default)]
struct Done {
    tags: Vec<IssueTag>,
    labels: Vec<IssueLabel>,
    parent: Option<IssueParent>,
    events: Vec<DomainEvent>,
}

fn extension(media_type: &str) -> &'static str {
    match media_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        _ => "pdf",
    }
}
