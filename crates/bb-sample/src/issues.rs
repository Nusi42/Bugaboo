//! The issues of the sample organization (architecture §11, *Projects*): about ten thousand
//! realistic issues over eighteen months with their sub-issues, relations, comments, tags and
//! labels, an edit lock held and a draft waiting. All of it is made through the use cases the API
//! calls, by the Team members of the sample, each signed in on the working days they work, one
//! change a moment, from the plan of [`crate::issues_plan`].
//!
//! The plan has no database in it; this is the part that carries it out. Everything that is random
//! comes from the seed, so the same seed makes the same issues, and the readable IDs of the
//! projects with *Random IDs* are the only thing that differs from run to run (the dump leaves
//! them out).
//!
//! The file is longer than 400 lines (coding-standards §2) because every kind of step is one
//! small method of the one `Maker` that holds the made issues and comments; the sessions and the
//! states left behind are already in files of their own.

use std::collections::BTreeMap;

use bb_app::comments::{AddComment, SaveComment, add_comment, save_comment};
use bb_app::context::Clock as _;
use bb_app::context::SystemIds;
use bb_app::editing::{TakeLockQuery, take_lock};
use bb_app::issues::{ChangeIssue, CreateIssue, change_issue, create_issue};
use bb_app::relations::{AddRelation, SetParent, add_relation, set_parent};
use bb_app::tags::{AddLabel, AddTag, add_label, add_tag};
use bb_core::comment::{TEXT_PART, handle_of};
use bb_core::edit::EditWhat;
use bb_core::event::{EntityRef, RecordKind};
use bb_core::fields::Field;
use bb_core::id::{CommentId, EditorId, IssueId, LockToken, UserId};
use bb_core::issue::StatusGroup;
use bb_core::relation::Direction;
use bb_core::technical::TechnicalName;
use bb_core::time::SECONDS_PER_DAY;
use bb_core::workflow::StatusKey;
use bb_dev_words::sample::{IssuePlan, Layout};
use serde_json::{Value, json};

use crate::error::{SampleError, Step};
use crate::generate::{Later, LeftBehind, World};
use crate::issues_hands::Hands;
use crate::issues_plan::{Action, FieldPick, Plan, Reached};
use crate::issues_setup::{Setup, id_in, json};
use crate::issues_time::Calendar;
use crate::issues_words::{IssueWords, MENTION};
use crate::organization::Organization;

/// The number of days in a month the history is counted in.
const MONTH_DAYS: i64 = 30;
/// How long before the sample ends the last working day ends, in seconds: room for the states the
/// sample leaves behind (a lock held, a draft waiting).
const END_ROOM: i64 = SECONDS_PER_DAY;

/// Everything the sample holds: the organization of M2 and the issues of M3.
#[derive(Clone, Debug)]
pub struct Everything {
    issues: u32,
    names: IssuePlan,
    left: LeftBehind,
}

impl Everything {
    /// The organization and `issues` issues with what belongs to them.
    ///
    /// # Errors
    /// [`SampleError::Layout`] when `tools/dev/sample.json` is not usable.
    pub fn new(issues: u32) -> Result<Self, SampleError> {
        Ok(Self {
            issues,
            names: Layout::bundled()?.issues,
            left: LeftBehind::default(),
        })
    }

    /// The full sample of `tools/dev/sample.json`: the demo and the browser tests.
    ///
    /// # Errors
    /// [`SampleError::Layout`] when `tools/dev/sample.json` is not usable.
    pub fn full() -> Result<Self, SampleError> {
        let total = Layout::bundled()?.issues.total;
        Self::new(total)
    }

    /// The smaller sample that `--check` makes four times.
    ///
    /// # Errors
    /// [`SampleError::Layout`] when `tools/dev/sample.json` is not usable.
    pub fn for_check() -> Result<Self, SampleError> {
        let total = Layout::bundled()?.issues.small_total;
        Self::new(total)
    }

    /// How many issues the sample is made with.
    #[must_use]
    pub const fn issues(&self) -> u32 {
        self.issues
    }
}

impl Later for Everything {
    fn history_seconds(&self) -> i64 {
        i64::from(self.names.months) * MONTH_DAYS * SECONDS_PER_DAY + END_ROOM
    }

    fn left_behind(&self) -> LeftBehind {
        self.left.clone()
    }

    async fn after_people(&mut self, world: &World<'_>) -> Result<(), SampleError> {
        Organization.after_people(world).await?;
        let names = self.names.clone();
        let words = IssueWords::bundled()?;
        let begins = world.clock.now().as_second() + 3600;
        let ends = world.start.as_second() - END_ROOM;
        let calendar = Calendar::between(begins, ends);
        let plan = Plan::new(&names, &words, world.seed, self.issues, &calendar)?;
        let token = world.administrator_session.to_owned();
        let setup = Setup::create(world, &token, &names).await?;
        let mut hands = Hands::new(world, calendar.clone(), plan.steps.len())?;
        let mut maker = Maker {
            world,
            plan: &plan,
            names: &names,
            setup: &setup,
            hands: &mut hands,
            issues: vec![None; plan.issues.len()],
            comments: vec![None; plan.comments.len()],
        };
        maker.carry_out(&calendar).await?;
        let work_since = world.clock.now();
        let full: u32 = names.projects.iter().map(|project| project.issues).sum();
        maker.work_together(self.issues, full).await?;
        self.left = maker.leave_states_behind().await?;
        self.left.work_since = Some(work_since);
        // What a server does every day (the job `history.catch-up`): the history of every issue
        // is read from the events, so the first page that shows one does not have to.
        world
            .services
            .ctx(world.request())
            .catch_up_history()
            .await
            .map_err(|error| SampleError::step(Step::History, &error))?;
        Ok(())
    }
}

/// Carries a plan out, step by step.
pub(crate) struct Maker<'a, 'w> {
    pub(crate) world: &'a World<'w>,
    pub(crate) plan: &'a Plan,
    pub(crate) names: &'a IssuePlan,
    pub(crate) setup: &'a Setup,
    pub(crate) hands: &'a mut Hands<'a, 'w>,
    /// The ID and the version of each issue that was made, and who made it.
    pub(crate) issues: Vec<Option<MadeIssue>>,
    /// The ID of each comment that was written, who wrote it and its version.
    pub(crate) comments: Vec<Option<MadeComment>>,
}

/// The most status changes it takes to get an issue into a status group: no starter workflow needs
/// more.
const STEPS_TO_A_GROUP_MAX: usize = 4;

#[derive(Clone, Debug)]
pub(crate) struct MadeIssue {
    pub(crate) id: IssueId,
    pub(crate) version: i64,
    /// The status group the issue is in.
    group: String,
    /// The status changes the person who made it may make now.
    pub(crate) changes: Vec<StatusOption>,
    /// The people it is assigned to, as far as the reader sees them.
    pub(crate) assigned: Vec<UserId>,
}

/// A status change an issue offers: the status it ends in, and the fields it still needs (*Ask for*
/// and *Must be filled in*), by their keys.
#[derive(Clone, Debug)]
pub(crate) struct StatusOption {
    pub(crate) key: String,
    pub(crate) group: String,
    pub(crate) needs: Vec<String>,
}

impl MadeIssue {
    /// What a status change that asks for the person who does the work carries: the one person who
    /// is assigned, else the person who makes the change takes it over (with two or more assigned
    /// the answer does not say in which order they are kept, so the change names one person
    /// instead). A change that asks for nothing of the kind carries nothing.
    pub(crate) fn assigned_for(
        &self,
        option: &StatusOption,
        person: UserId,
    ) -> Option<Vec<UserId>> {
        option
            .needs_a_person()
            .then(|| match self.assigned.as_slice() {
                [one] => vec![*one],
                _ => vec![person],
            })
    }
}

impl StatusOption {
    /// Whether the change needs a person to be assigned.
    pub(crate) fn needs_a_person(&self) -> bool {
        self.needs
            .iter()
            .any(|field| field == Field::AssignedTo.code())
    }
}

impl MadeIssue {
    /// The issue as an answer of the use cases shows it.
    pub(crate) fn of(answer: &Value) -> Result<Self, SampleError> {
        let text = |value: &Value| value.as_str().unwrap_or_default().to_owned();
        Ok(Self {
            id: id_in(answer, "id")?,
            version: answer["version"].as_i64().unwrap_or(1),
            group: text(&answer["status"]["group"]),
            assigned: answer["assigned_to"]
                .as_array()
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .filter_map(|person| person["id"].as_str()?.parse().ok())
                .collect(),
            changes: answer["status_changes"]
                .as_array()
                .map(|changes| {
                    changes
                        .iter()
                        .map(|each| StatusOption {
                            key: text(&each["to"]["key"]),
                            group: text(&each["to"]["group"]),
                            needs: each["needs"]
                                .as_array()
                                .map_or(&[][..], Vec::as_slice)
                                .iter()
                                .map(text)
                                .collect(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
    }

    /// The status change that goes toward the group, if there is one: one that ends in it, else
    /// one that ends in work going on while the issue has not started.
    fn toward(&self, reached: StatusGroup) -> Option<&StatusOption> {
        let ends_in =
            |group: StatusGroup| self.changes.iter().find(|each| each.group == group.code());
        if self.group == reached.code() {
            return None;
        }
        ends_in(reached).or_else(|| {
            (self.group == StatusGroup::NotStarted.code())
                .then(|| ends_in(StatusGroup::InProgress))
                .flatten()
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MadeComment {
    pub(crate) id: CommentId,
    pub(crate) author: usize,
    version: i64,
}

impl Maker<'_, '_> {
    pub(crate) fn issue(&self, at: usize) -> Result<&MadeIssue, SampleError> {
        self.issues
            .get(at)
            .and_then(Option::as_ref)
            .ok_or_else(|| SampleError::NotMade(format!("issue {at}")))
    }

    fn version_up(&mut self, at: usize) {
        if let Some(Some(made)) = self.issues.get_mut(at) {
            made.version += 1;
        }
    }

    /// The people, as the people of the plan of the people, whom picks stand for.
    fn assigned_people(&self, picks: &[u32]) -> Vec<UserId> {
        let mut people: Vec<UserId> = Vec::new();
        for pick in picks {
            let person = self.world.people.get(self.hands.anyone(*pick));
            if let Some(made) = person.filter(|made| !people.contains(&made.id)) {
                people.push(made.id);
            }
        }
        people
    }

    /// A text with the marker of a mention replaced by the mention of a person.
    fn mentioning(&self, text: &str, pick: u32) -> String {
        let handle = self
            .world
            .people
            .get(self.hands.anyone(pick))
            .and_then(|made| handle_of(&made.person.name))
            .unwrap_or_default();
        text.replace(MENTION, &format!("@{handle}"))
    }

    /// The value of an own field, as the use case takes it.
    fn field_value(&self, field: usize, pick: &FieldPick) -> Option<Value> {
        let made = self.setup.fields.get(field)?;
        Some(match pick {
            FieldPick::Text(text) => json!(text),
            FieldPick::Choices(at) => {
                let ids: Vec<String> = at
                    .iter()
                    .filter_map(|at| made.choices.get(*at))
                    .map(ToString::to_string)
                    .collect();
                if made.kind == bb_core::own_fields::FieldType::PickOne {
                    json!(ids.first()?)
                } else {
                    json!(ids)
                }
            }
        })
    }

    async fn carry_out(&mut self, calendar: &Calendar) -> Result<(), SampleError> {
        let mut last = self.world.clock.now().as_second();
        let working: Vec<i64> = self.plan.steps.iter().map(|step| step.at).collect();
        let moments = calendar.pack(&working);
        for (step, planned) in self.plan.steps.iter().zip(moments) {
            let moment = planned.max(last + 1);
            last = moment;
            self.world
                .clock
                .set(jiff::Timestamp::from_second(moment).map_err(|_| SampleError::Start(moment))?);
            let person = self.hands.person(step.at, step.lane);
            match step.action {
                Action::Create(at) => self.create(at, person).await?,
                Action::Parent(at) => self.parent(at, person).await?,
                Action::Tag(at, tag) => self.tag(at, tag, person).await?,
                Action::Label(at, label) => self.label(at, label, person).await?,
                Action::Status(at, reached) => self.status(at, reached, person).await?,
                Action::Comment(at) => self.comment(at, person).await?,
                Action::Edit(at) => self.edit(at).await?,
                Action::Relate(at) => self.relate(at, person).await?,
            }
        }
        Ok(())
    }

    async fn create(&mut self, at: usize, person: usize) -> Result<(), SampleError> {
        let planned = &self.plan.issues[at];
        let token = self.hands.token(person).await?;
        let assigned = self.assigned_people(&planned.assigned_people);
        let ordered = assigned.clone();
        let fields = planned
            .fields
            .iter()
            .enumerate()
            .filter_map(|(field, pick)| {
                let made = self.setup.fields.get(field)?;
                Some((
                    made.id.to_string(),
                    self.field_value(field, pick.as_ref()?)?,
                ))
            })
            .collect();
        let made = json(
            create_issue(
                &self.world.services.ctx(self.world.request()),
                CreateIssue {
                    session_token: token,
                    project: planned.place.clone(),
                    issue_type: *self
                        .setup
                        .types
                        .get(planned.kind)
                        .ok_or_else(|| SampleError::NotMade(planned.title.clone()))?,
                    title: planned.title.clone(),
                    description: Some(planned.description.clone()),
                    status: None,
                    priority: Some(planned.priority),
                    assigned_to: (!assigned.is_empty()).then_some(assigned),
                    fields,
                },
            )
            .await,
            Step::Issue,
        )?;
        let mut created = MadeIssue::of(&made)?;
        created.assigned = ordered;
        self.issues[at] = Some(created);
        Ok(())
    }

    async fn parent(&mut self, at: usize, person: usize) -> Result<(), SampleError> {
        let parent = self.plan.issues[at]
            .parent
            .ok_or_else(|| SampleError::NotMade(format!("parent of {at}")))?;
        let (child, goal) = (self.issue(at)?.clone(), self.issue(parent)?.clone());
        let token = self.hands.token(person).await?;
        json(
            set_parent(
                &self.world.services.ctx(self.world.request()),
                SetParent {
                    session_token: token,
                    issue: child.id.to_string(),
                    version: child.version,
                    parent: Some(goal.id.to_string()),
                },
            )
            .await,
            Step::Relation,
        )?;
        self.version_up(at);
        Ok(())
    }

    async fn tag(&mut self, at: usize, tag: usize, person: usize) -> Result<(), SampleError> {
        let id = self.issue(at)?.id;
        let name = self.names.tags[tag].clone();
        let token = self.hands.token(person).await?;
        add_tag(
            &self.world.services.ctx(self.world.request()),
            AddTag {
                session_token: token,
                issue: id.to_string(),
                name,
            },
        )
        .await
        .map_err(|error| SampleError::step(Step::Tags, &error))?;
        self.version_up(at);
        Ok(())
    }

    async fn label(&mut self, at: usize, label: usize, person: usize) -> Result<(), SampleError> {
        let id = self.issue(at)?.id;
        let label = *self
            .setup
            .labels
            .get(label)
            .ok_or_else(|| SampleError::NotMade(format!("label {label}")))?;
        let token = self.hands.token(person).await?;
        add_label(
            &self.world.services.ctx(self.world.request()),
            AddLabel {
                session_token: token,
                issue: id.to_string(),
                label,
            },
        )
        .await
        .map_err(|error| SampleError::step(Step::Tags, &error))?;
        self.version_up(at);
        Ok(())
    }

    async fn status(
        &mut self,
        at: usize,
        reached: Reached,
        person: usize,
    ) -> Result<(), SampleError> {
        // The workflow of the issue says which status changes there are: walk them toward the group.
        for _ in 0..STEPS_TO_A_GROUP_MAX {
            let made = self.issue(at)?.clone();
            let Some(next) = made.toward(reached.group()) else {
                break;
            };
            // The session is taken only when a request follows: a session that is marked as used
            // and is not used would run out of idle time on the server unseen.
            let token = self.hands.token(person).await?;
            let carried = made.assigned_for(next, self.world.people[person].id);
            let changed = json(
                change_issue(
                    &self.world.services.ctx(self.world.request()),
                    ChangeIssue {
                        session_token: token,
                        issue: made.id.to_string(),
                        version: made.version,
                        issue_type: None,
                        title: None,
                        status: Some(
                            StatusKey::new(&next.key)
                                .map_err(|error| SampleError::NotMade(error.to_string()))?,
                        ),
                        priority: None,
                        // A change that asks for the person who does the work: the person who
                        // makes it does.
                        assigned_to: carried.clone(),
                        fields: BTreeMap::new(),
                    },
                )
                .await,
                Step::Change,
            )?;
            let mut updated = MadeIssue::of(&changed)?;
            updated.assigned = carried.unwrap_or(made.assigned);
            self.issues[at] = Some(updated);
        }
        Ok(())
    }

    async fn comment(&mut self, at: usize, person: usize) -> Result<(), SampleError> {
        let planned = &self.plan.comments[at];
        let issue = self.issue(planned.issue)?.id;
        let text = self.mentioning(&planned.text, planned.mention);
        let token = self.hands.token(person).await?;
        let written = json(
            add_comment(
                &self.world.services.ctx(self.world.request()),
                AddComment {
                    session_token: token,
                    issue: issue.to_string(),
                    text,
                },
            )
            .await,
            Step::Comment,
        )?;
        self.comments[at] = Some(MadeComment {
            id: id_in(&written, "id")?,
            author: person,
            version: written["version"].as_i64().unwrap_or(1),
        });
        Ok(())
    }

    /// The author of a comment takes the lock on its text, changes it and saves.
    async fn edit(&mut self, at: usize) -> Result<(), SampleError> {
        let planned = &self.plan.comments[at];
        let Some(edit) = &planned.edit else {
            return Ok(());
        };
        let made = self
            .comments
            .get(at)
            .and_then(Option::as_ref)
            .ok_or_else(|| SampleError::NotMade(format!("comment {at}")))?
            .clone();
        let text = self.mentioning(&edit.text, planned.mention);
        let token = self.hands.token(made.author).await?;
        let ctx = self.world.services.ctx(self.world.request());
        let what = EditWhat {
            record: EntityRef {
                kind: RecordKind::Comment.into(),
                id: made.id.into(),
            },
            part: TechnicalName::new(TEXT_PART)
                .map_err(|error| SampleError::step(Step::Editing, &error))?,
        };
        let outcome = json(
            take_lock(
                &ctx,
                TakeLockQuery {
                    session_token: token.clone(),
                    what,
                    editor: EditorId::generate(&mut SystemIds),
                },
            )
            .await,
            Step::Editing,
        )?;
        if outcome["status"] != "taken" {
            return Err(SampleError::NotMade(format!("lock of comment {at}")));
        }
        let lock_token: LockToken = id_in(&outcome, "token")?;
        json(
            save_comment(
                &ctx,
                SaveComment {
                    session_token: token,
                    comment: made.id,
                    version: made.version,
                    lock_token,
                    text,
                },
            )
            .await,
            Step::Editing,
        )?;
        Ok(())
    }

    async fn relate(&mut self, at: usize, person: usize) -> Result<(), SampleError> {
        let planned = self.plan.relations[at];
        let (from, to) = (self.issue(planned.from)?.id, self.issue(planned.to)?.id);
        let kind = *self
            .setup
            .kinds
            .get(planned.kind)
            .ok_or_else(|| SampleError::NotMade(format!("kind {}", planned.kind)))?;
        let token = self.hands.token(person).await?;
        json(
            add_relation(
                &self.world.services.ctx(self.world.request()),
                AddRelation {
                    session_token: token,
                    issue: from.to_string(),
                    other: to.to_string(),
                    kind,
                    direction: Direction::Forward,
                },
            )
            .await,
            Step::Relation,
        )?;
        Ok(())
    }
}
