//! The people, groups and projects of the large dataset (architecture §11, *Large dataset*): the
//! sample's fifty people and four projects with their sub-projects stay as the use cases made them;
//! the rest up to the numbers of `large` in `tools/dev/sample.json` is written in one
//! transaction. The people written this way cannot sign in (their password is thrown away); the
//! fifty of `sign-in.txt` can.

use std::collections::BTreeSet;

use bb_app::access::{PolicyStore as _, StartingRole};
use bb_app::auth::AccountStore as _;
use bb_app::context::Clock as _;
use bb_app::event_members;
use bb_app::projects::ProjectStore as _;
use bb_auth::PasswordHasher;
use bb_core::access::{AppliesTo, Assignment, Group, GroupMember};
use bb_core::comment::handle_of;
use bb_core::event::{DomainEvent, RecordKind};
use bb_core::id::{AssignmentId, EntityId, GroupId, ProjectId, RoleId, UserId};
use bb_core::person::Person;
use bb_core::project::{Project, ShortName};
use bb_core::technical::TechnicalName;
use bb_core::version::FIRST_VERSION;
use bb_dev_words::LargePlan;
use bb_dev_words::sample::Layout;
use bb_store::AuthStorage;
use jiff::Timestamp;
use serde::Deserialize;
use zeroize::Zeroizing;

use crate::error::{SampleError, Step};
use crate::generate::World;
use crate::large_build::event;
use crate::large_ids::{ordered, random16};
use crate::names::NameList;
use crate::organization::workers;
use crate::people_plan::{EMAIL_DOMAIN, ROLE_MIX};
use crate::random::SeededRandom;

const WORDS: &str = include_str!("../data/large.toml");

/// How many of 100 Team members of the rest work in the whole organization (the others hold
/// their role in a few projects, directly or through a group).
const TEAM_EVERYWHERE_PERCENT: usize = 40;
/// The most families a Team member of the rest works in.
const FAMILIES_PER_TEAM_MEMBER: usize = 3;
/// The most families a Contributor works in.
const FAMILIES_PER_CONTRIBUTOR: usize = 2;
/// The areas a new family is spread over (the keys of the words of `issues.toml`).
const AREAS: [&str; 4] = ["app", "web", "ops", "doc"];

#[derive(Deserialize)]
struct Words {
    projects: ProjectWords,
}

#[derive(Deserialize)]
struct ProjectWords {
    nouns: Vec<String>,
    topics: Vec<String>,
    teams: Vec<String>,
}

/// A person of the dataset and the role they hold in most places.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Folk {
    pub(crate) id: UserId,
    pub(crate) role: StartingRole,
}

/// A project that holds issues, and who may do what in it.
#[derive(Clone, Debug)]
pub(crate) struct Place {
    pub(crate) id: ProjectId,
    pub(crate) short_name: String,
    pub(crate) name: String,
    /// The project itself, then the projects above it.
    pub(crate) chain: Vec<ProjectId>,
    /// The words its issues are made from (a key of the lists of `issues.toml`).
    pub(crate) area: String,
    pub(crate) random_codes: bool,
    pub(crate) german: bool,
    /// How much of the issues the project gets, in parts per million of all.
    pub(crate) weight: u64,
    /// The people (places in [`Org::folk`]) who work here besides the ones who work everywhere.
    pub(crate) team: Vec<usize>,
    /// The Contributors and Guests who hold a role here.
    pub(crate) outsiders: Vec<usize>,
}

/// Everybody and every project of the dataset.
#[derive(Debug)]
pub(crate) struct Org {
    pub(crate) folk: Vec<Folk>,
    /// The people who work in every project: Administrators and Team members of the whole organization.
    pub(crate) everywhere: Vec<usize>,
    pub(crate) places: Vec<Place>,
}

/// A family of projects the rest is made of: a project and the sub-projects in it.
struct Family {
    parent: usize,
}

struct Roles {
    ids: Vec<(StartingRole, RoleId)>,
}

impl Roles {
    async fn read(storage: &AuthStorage) -> Result<Self, SampleError> {
        let mut ids = Vec::new();
        for record in storage.roles().await? {
            let starting = record.role.starting.as_ref();
            let found = ROLE_MIX
                .iter()
                .find(|(role, _)| Some(role.code()) == starting.map(TechnicalName::as_str));
            if let Some((role, _)) = found {
                ids.push((*role, record.role.id));
            }
        }
        Ok(Self { ids })
    }

    fn of(&self, role: StartingRole) -> Result<RoleId, SampleError> {
        self.ids
            .iter()
            .find(|(each, _)| *each == role)
            .map(|(_, id)| *id)
            .ok_or_else(|| SampleError::NotMade(role.code().to_owned()))
    }
}

/// How many people of each role the rest has: the mix of the sample, scaled to `people` in all.
fn rest_of_the_mix(people: u32) -> Vec<(StartingRole, usize)> {
    let sample: usize = ROLE_MIX.iter().map(|(_, count)| count).sum();
    let rest = usize::try_from(people).unwrap_or(0).saturating_sub(sample);
    ROLE_MIX
        .iter()
        .map(|(role, count)| (*role, count * rest / sample))
        .collect()
}

/// Splits `total` into the weights of `count` projects: the first tenth, the big ones, gets
/// `big_share` percent, shared by 1 / rank; the rest shares what is left the same way.
fn weights(count: usize, big_share: u32) -> Vec<u64> {
    const WHOLE: u64 = 1_000_000;
    let big = (count / 10).max(1);
    // The weight of the k-th project of a tier is 1 / k, in millionths.
    let tier = |len: usize, share: u64| -> Vec<u64> {
        let parts: Vec<u64> = (1..=len as u64).map(|rank| WHOLE / rank).collect();
        let sum: u64 = parts.iter().sum::<u64>().max(1);
        parts.iter().map(|part| share * part / sum).collect()
    };
    let share = u64::from(big_share) * (WHOLE / 100);
    let mut found = tier(big, share);
    if count > big {
        found.extend(tier(count - big, WHOLE - share));
    }
    found
}

/// Who holds which role where, as the rows to write.
struct Rows {
    /// A person, the role, and the family (its first project) the role counts in; none is the
    /// whole organization.
    direct: Vec<(usize, StartingRole, Option<usize>)>,
    /// The Team members of each family, which a group of that family holds.
    team_of_family: Vec<Vec<usize>>,
}

/// The projects the sample made, with what the layout says of their words and IDs.
fn sample_places(tree: &[Project], layout: &Layout) -> Vec<Place> {
    tree.iter()
        .map(|project| {
            let mut chain = vec![project.id];
            let mut above = project.parent;
            while let Some(parent) = above {
                chain.push(parent);
                above = tree
                    .iter()
                    .find(|each| each.id == parent)
                    .and_then(|each| each.parent);
            }
            let family = chain.last().copied().unwrap_or(project.id);
            let top = tree.iter().find(|each| each.id == family);
            let top_name = top.map_or(project.short_name.as_str(), |top| top.short_name.as_str());
            let issues = layout
                .issues
                .projects
                .iter()
                .find(|each| each.short_name == top_name);
            Place {
                id: project.id,
                short_name: project.short_name.as_str().to_owned(),
                name: project.name.clone(),
                chain,
                area: issues.map_or("app", |each| each.area.as_str()).to_owned(),
                random_codes: issues.is_some_and(|each| each.id_mode == "random"),
                german: false,
                weight: 0,
                team: Vec::new(),
                outsiders: Vec::new(),
            }
        })
        .collect()
}

/// Adds families of projects up to the number the plan asks for; each is a project and up to
/// `subProjectsEach` sub-projects of it.
fn add_families(
    places: &mut Vec<Place>,
    plan: &LargePlan,
    words: &ProjectWords,
    random: &mut SeededRandom,
    at: Timestamp,
) -> Result<Vec<Family>, SampleError> {
    let wanted = usize::try_from(plan.projects).unwrap_or(0);
    let most = usize::try_from(plan.sub_projects_each).unwrap_or(0) + 1;
    let mut families = Vec::new();
    let mut family_no = 0_usize;
    while places.len() < wanted {
        family_no += 1;
        let area = AREAS[random.below(AREAS.len())];
        let german = random.chance(plan.german_percent);
        let parent = places.len();
        let subs = random.below(most).min(wanted - parent - 1);
        let noun = random.pick(&words.nouns).cloned().unwrap_or_default();
        let topic = random.pick(&words.topics).cloned().unwrap_or_default();
        families.push(Family { parent });
        for number in 0..=subs {
            let letter = char::from(b'A' + u8::try_from(number.saturating_sub(1)).unwrap_or(0));
            let id = ordered(ProjectId::from_parts, at, random)?;
            let (short_name, name, chain) = if number == 0 {
                (
                    format!("P{family_no:03}"),
                    format!("{noun} {topic}"),
                    vec![id],
                )
            } else {
                (
                    format!("P{family_no:03}{letter}"),
                    format!("{noun} {topic} {number}"),
                    vec![id, places[parent].id],
                )
            };
            places.push(Place {
                id,
                short_name,
                name,
                chain,
                area: area.to_owned(),
                random_codes: false,
                german,
                weight: 0,
                team: Vec::new(),
                outsiders: Vec::new(),
            });
        }
    }
    Ok(families)
}

/// The people of the rest: each a given and a family name of the list that nobody has yet.
fn add_people(
    world: &World<'_>,
    plan: &LargePlan,
    random: &mut SeededRandom,
    at: Timestamp,
) -> Result<Vec<(Person, StartingRole)>, SampleError> {
    let names = NameList::bundled()?;
    let mut used: BTreeSet<String> = world
        .people
        .iter()
        .map(|made| made.person.name.clone())
        .collect();
    let mut people = Vec::new();
    for (role, count) in rest_of_the_mix(plan.people) {
        for _ in 0..count {
            let (first, last) = loop {
                let first = random.pick(names.first()).cloned().unwrap_or_default();
                let last = random.pick(names.last()).cloned().unwrap_or_default();
                if used.insert(format!("{first} {last}")) {
                    break (first, last);
                }
            };
            people.push((
                Person {
                    id: UserId::from_random(random16(random)),
                    display_name: format!("{first} {last}"),
                    email: format!(
                        "{}.{}@{EMAIL_DOMAIN}",
                        first.to_ascii_lowercase(),
                        last.to_ascii_lowercase()
                    ),
                    created_at: at,
                },
                role,
            ));
        }
    }
    Ok(people)
}

fn push_into(places: &mut [Place], parent: usize, index: usize, outsider: bool) {
    let family = places[parent].id;
    for place in places
        .iter_mut()
        .filter(|place| place.chain.last() == Some(&family))
    {
        let list = if outsider {
            &mut place.outsiders
        } else {
            &mut place.team
        };
        if !list.contains(&index) {
            list.push(index);
        }
    }
}

/// Says who holds which role where, for the people of the rest (from `first_new` on).
fn assign(org: &mut Org, families: &[Family], first_new: usize, random: &mut SeededRandom) -> Rows {
    let family_count = families.len().max(1);
    let mut rows = Rows {
        direct: Vec::new(),
        team_of_family: vec![Vec::new(); family_count],
    };
    let family_of = |random: &mut SeededRandom| {
        let at = random.below(family_count);
        families.get(at).map(|family| (at, family.parent))
    };
    for index in first_new..org.folk.len() {
        let role = org.folk[index].role;
        match role {
            StartingRole::Administrator => {
                rows.direct.push((index, role, None));
                org.everywhere.push(index);
            }
            StartingRole::Viewer => rows.direct.push((index, role, None)),
            StartingRole::ProjectLead => {
                if let Some((_, parent)) = family_of(random) {
                    rows.direct.push((index, role, Some(parent)));
                    push_into(&mut org.places, parent, index, false);
                }
            }
            StartingRole::TeamMember
                if random.chance(u32::try_from(TEAM_EVERYWHERE_PERCENT).unwrap_or(0)) =>
            {
                rows.direct.push((index, role, None));
                org.everywhere.push(index);
            }
            StartingRole::TeamMember => {
                for _ in 0..=random.below(FAMILIES_PER_TEAM_MEMBER) {
                    if let Some((at, parent)) = family_of(random) {
                        rows.team_of_family[at].push(index);
                        rows.direct.push((index, role, Some(parent)));
                        push_into(&mut org.places, parent, index, false);
                    }
                }
            }
            StartingRole::Contributor | StartingRole::Guest => {
                let most = if role == StartingRole::Guest {
                    1
                } else {
                    FAMILIES_PER_CONTRIBUTOR
                };
                for _ in 0..=random.below(most) {
                    if let Some((_, parent)) = family_of(random) {
                        rows.direct.push((index, role, Some(parent)));
                        push_into(&mut org.places, parent, index, true);
                    }
                }
            }
        }
    }
    rows
}

/// The weights of the projects: the sample's projects are among the biggest.
fn rank(places: &mut [Place], sample: usize, plan: &LargePlan, random: &mut SeededRandom) {
    let shares = weights(places.len(), plan.big_projects_share);
    let mut order: Vec<usize> = (0..places.len()).collect();
    random.shuffle(&mut order[sample..]);
    for (rank, place) in order.into_iter().enumerate() {
        places[place].weight = shares.get(rank).copied().unwrap_or(1);
    }
}

/// Everything the transaction writes, with the events that go with it.
struct Written {
    people: Vec<Person>,
    projects: Vec<Project>,
    groups: Vec<(Group, Vec<GroupMember>, Assignment)>,
    rows: Vec<Assignment>,
}

fn role_rows(
    org: &Org,
    rows: &Rows,
    roles: &Roles,
    given_at: Timestamp,
    random: &mut SeededRandom,
) -> Result<Vec<Assignment>, SampleError> {
    rows.direct
        .iter()
        .map(|(index, role, family_parent)| {
            let project = family_parent.map(|parent| org.places[parent].id);
            Ok(Assignment {
                id: ordered(AssignmentId::from_parts, given_at, random)?,
                person: Some(org.folk[*index].id),
                group: None,
                role: Some(roles.of(*role)?),
                project,
                applies_to: if project.is_some() {
                    AppliesTo::ProjectAndSubProjects
                } else {
                    AppliesTo::Organization
                },
                given_at,
            })
        })
        .collect()
}

/// The rest of the groups: each is a team of one family, with the role *Team member* there.
fn group_rows(
    org: &Org,
    rows: &Rows,
    families: &[Family],
    roles: &Roles,
    count: usize,
    (words, given_at): (&ProjectWords, Timestamp),
    random: &mut SeededRandom,
) -> Result<Vec<(Group, Vec<GroupMember>, Assignment)>, SampleError> {
    let role = roles.of(StartingRole::TeamMember)?;
    let family_count = families.len().max(1);
    (0..count)
        .map(|at_group| {
            let team = random.pick(&words.teams).cloned().unwrap_or_default();
            let group = Group {
                id: ordered(GroupId::from_parts, given_at, random)?,
                name: format!("Team {team} {}", at_group + 1),
                version: FIRST_VERSION,
            };
            let family = at_group % family_count;
            let members = rows.team_of_family[family]
                .iter()
                .map(|member| GroupMember {
                    group: group.id,
                    person: org.folk[*member].id,
                    added_at: given_at,
                })
                .collect();
            let project = families.get(family).map(|each| org.places[each.parent].id);
            let assignment = Assignment {
                id: ordered(AssignmentId::from_parts, given_at, random)?,
                person: None,
                group: Some(group.id),
                role: Some(role),
                project,
                applies_to: AppliesTo::ProjectAndSubProjects,
                given_at,
            };
            Ok((group, members, assignment))
        })
        .collect()
}

/// The events of everything that is written: who made it is the first administrator, and a person
/// who joined made their own account.
fn events_of(
    written: &Written,
    administrator: UserId,
    at: Timestamp,
    random: &mut SeededRandom,
) -> Result<Vec<DomainEvent>, SampleError> {
    let mut found = Vec::new();
    for person in &written.people {
        found.push(event(
            random,
            at,
            person.id,
            RecordKind::Person,
            person.id,
            event_members::person_joined(),
        )?);
    }
    let mut add = |kind, id: EntityId, changes| -> Result<(), SampleError> {
        found.push(event(random, at, administrator, kind, id, changes)?);
        Ok(())
    };
    for project in &written.projects {
        add(
            RecordKind::Project,
            project.id.into(),
            event_members::project_created(project),
        )?;
    }
    for (group, members, assignment) in &written.groups {
        add(
            RecordKind::Group,
            group.id.into(),
            event_members::group_created(group),
        )?;
        add(
            RecordKind::Assignment,
            assignment.id.into(),
            event_members::assignment_given(assignment),
        )?;
        for member in members {
            add(
                RecordKind::Group,
                member.group.into(),
                event_members::group_member_added(),
            )?;
        }
    }
    for row in &written.rows {
        add(
            RecordKind::Assignment,
            row.id.into(),
            event_members::assignment_given(row),
        )?;
    }
    Ok(found)
}

/// Writes everything in one transaction, with the events.
async fn store(
    storage: &AuthStorage,
    written: &Written,
    events: &[DomainEvent],
    reserved: &[String],
    at: Timestamp,
) -> Result<(), SampleError> {
    let hash = PasswordHasher::new()
        .map_err(|error| SampleError::step(Step::PasswordHasher, &error))?
        .hash(Zeroizing::new(crate::password::random_password()?))
        .await
        .map_err(|error| SampleError::step(Step::PasswordHasher, &error))?;
    storage
        .database()
        .record_changes(events, async |transaction| {
            for person in &written.people {
                AuthStorage::insert_person(transaction, person, &hash, at).await?;
                event_members::give_handle::<AuthStorage>(transaction, person, at, reserved)
                    .await?;
            }
            for project in &written.projects {
                AuthStorage::insert_project(transaction, project).await?;
            }
            for (group, members, assignment) in &written.groups {
                AuthStorage::insert_group(transaction, group).await?;
                AuthStorage::insert_assignment(transaction, assignment).await?;
                for member in members {
                    AuthStorage::insert_group_member(transaction, member).await?;
                }
            }
            for row in &written.rows {
                AuthStorage::insert_assignment(transaction, row).await?;
            }
            Ok(())
        })
        .await?;
    Ok(())
}

/// Writes the rest of the people, groups and projects with their events, and says who works where.
///
/// # Errors
/// [`SampleError`] when a name list is unusable or a write fails.
pub(crate) async fn grow(
    world: &World<'_>,
    plan: &LargePlan,
    layout: &Layout,
) -> Result<Org, SampleError> {
    let words: Words =
        toml::from_str(WORDS).map_err(|error| SampleError::Names(error.to_string()))?;
    let mut random = SeededRandom::new(world.seed ^ 0x004C_4152_4745);
    let storage = world.storage;
    let at = world.clock.now();
    let roles = Roles::read(storage).await?;
    let tree = storage.project_tree().await?;
    let mut places = sample_places(&tree, layout);
    let sample_projects = places.len();
    let families = add_families(&mut places, plan, &words.projects, &mut random, at)?;
    let people_and_roles = add_people(world, plan, &mut random, at)?;
    let first_new = world.people.len();
    let mut org = Org {
        folk: world
            .people
            .iter()
            .map(|made| Folk {
                id: made.id,
                role: made.person.role,
            })
            .collect(),
        everywhere: workers(world)?,
        places,
    };
    let mut new_people = Vec::new();
    for (person, role) in people_and_roles {
        org.folk.push(Folk {
            id: person.id,
            role,
        });
        new_people.push(person);
    }
    let rows = assign(&mut org, &families, first_new, &mut random);
    rank(&mut org.places, sample_projects, plan, &mut random);

    let existing = storage.groups().await?;
    let reserved: Vec<String> = existing
        .iter()
        .map(|group| group.name.as_str())
        .chain(
            storage
                .roles()
                .await?
                .iter()
                .map(|record| record.role.name.as_str()),
        )
        .filter_map(handle_of)
        .collect();
    let group_count = usize::try_from(plan.groups)
        .unwrap_or(0)
        .saturating_sub(existing.len());
    let written = Written {
        people: new_people,
        projects: org.places[sample_projects..]
            .iter()
            .map(|place| {
                Ok(Project {
                    id: place.id,
                    parent: place.chain.get(1).copied(),
                    short_name: ShortName::new(&place.short_name)
                        .map_err(|error| SampleError::step(Step::Dataset, &error))?,
                    name: place.name.clone(),
                    version: FIRST_VERSION,
                })
            })
            .collect::<Result<_, SampleError>>()?,
        groups: group_rows(
            &org,
            &rows,
            &families,
            &roles,
            group_count,
            (&words.projects, at),
            &mut random,
        )?,
        rows: role_rows(&org, &rows, &roles, at, &mut random)?,
    };
    let administrator = world.people.first().map_or(org.folk[0].id, |made| made.id);
    let events = events_of(&written, administrator, at, &mut random)?;
    store(storage, &written, &events, &reserved, at).await?;
    org.everywhere.sort_unstable();
    org.everywhere.dedup();
    Ok(org)
}
