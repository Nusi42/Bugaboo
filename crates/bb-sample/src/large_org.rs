//! The people, groups and projects of the large dataset (architecture §11, *Large dataset*): the
//! sample's fifty people and four projects with their sub-projects stay as the use cases made them;
//! the rest up to the numbers of `large` in `tools/dev/sample.json` is written in one
//! transaction. The people written this way cannot sign in (their password is thrown away); the
//! fifty of `sign-in.txt` can.

use std::collections::BTreeSet;

use bb_app::access::{PolicyStore as _, StartingRole};
use bb_app::auth::AccountStore as _;
use bb_app::context::Clock as _;
use bb_app::projects::ProjectStore as _;
use bb_auth::PasswordHasher;
use bb_core::access::{AppliesTo, Assignment, Group, GroupMember};
use bb_core::comment::handle_of;
use bb_core::id::{AssignmentId, GroupId, ProjectId, RoleId, UserId};
use bb_core::person::Person;
use bb_core::project::{Project, ShortName};
use bb_core::version::FIRST_VERSION;
use bb_dev_words::LargePlan;
use bb_dev_words::sample::Layout;
use bb_store::AuthStorage;
use serde::Deserialize;
use zeroize::Zeroizing;

use crate::error::{SampleError, Step};
use crate::generate::World;
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
    const WHOLE: f64 = 1_000_000.0;
    let big = (count / 10).max(1);
    let tier = |from: usize, to: usize, share: f64| -> Vec<u64> {
        let harmonic: f64 = (from..to).map(|rank| 1.0 / (rank - from + 1) as f64).sum();
        (from..to)
            .map(|rank| (WHOLE * share / harmonic / (rank - from + 1) as f64).round() as u64)
            .collect()
    };
    let share = f64::from(big_share) / 100.0;
    let mut found = tier(0, big, share);
    if count > big {
        found.extend(tier(big, count, 1.0 - share));
    }
    found
}

/// Writes the rest of the people, groups and projects, and says who works where.
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
    let names = NameList::bundled()?;
    let mut random = SeededRandom::new(world.seed ^ 0x4C41_5247_45);
    let storage = world.storage;

    let roles = Roles {
        ids: {
            let mut found = Vec::new();
            for record in storage.roles().await? {
                if let Some(starting) = record.role.starting.as_ref() {
                    if let Some((role, _)) = ROLE_MIX
                        .iter()
                        .find(|(role, _)| role.code() == starting.as_str())
                    {
                        found.push((*role, record.role.id));
                    }
                }
            }
            found
        },
    };

    // The sample's people keep their places; those who work everywhere are the first workers.
    let mut folk: Vec<Folk> = world
        .people
        .iter()
        .map(|made| Folk {
            id: made.id,
            role: made.person.role,
        })
        .collect();
    let mut everywhere: Vec<usize> = workers(world)?;

    // The projects the sample made.
    let tree = storage.project_tree().await?;
    let mut places: Vec<Place> = Vec::new();
    for project in &tree {
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
        places.push(Place {
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
        });
    }

    // The families of the rest.
    let mut families: Vec<Family> = Vec::new();
    let wanted = usize::try_from(plan.projects).unwrap_or(0);
    let mut family_no = 0_usize;
    while places.len() < wanted {
        family_no += 1;
        let area = AREAS[random.below(AREAS.len())];
        let german = random.chance(plan.german_percent);
        let parent = places.len();
        let subs = random.below(usize::try_from(plan.sub_projects_each).unwrap_or(0) + 1);
        let subs = subs.min(wanted - parent - 1);
        let noun = random
            .pick(&words.projects.nouns)
            .cloned()
            .unwrap_or_default();
        let topic = random
            .pick(&words.projects.topics)
            .cloned()
            .unwrap_or_default();
        families.push(Family { parent });
        for at in 0..=subs {
            let letter = char::from(b'A' + u8::try_from(at.saturating_sub(1)).unwrap_or(0));
            let (short, name, above) = if at == 0 {
                (format!("P{family_no:03}"), format!("{noun} {topic}"), None)
            } else {
                (
                    format!("P{family_no:03}{letter}"),
                    format!("{noun} {topic} {at}"),
                    Some(places[parent].id),
                )
            };
            let id = ordered(ProjectId::from_parts, world.clock.now(), &mut random)?;
            let mut chain = vec![id];
            chain.extend(above);
            places.push(Place {
                id,
                short_name: short,
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
    let new_from = tree.len();
    let new_projects: Vec<Project> = places[new_from..]
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
        .collect::<Result<_, SampleError>>()?;

    // The people of the rest.
    let rest = rest_of_the_mix(plan.people);
    let mut used: BTreeSet<String> = world
        .people
        .iter()
        .map(|made| made.person.name.clone())
        .collect();
    let hash = PasswordHasher::new()
        .map_err(|error| SampleError::step(Step::PasswordHasher, &error))?
        .hash(Zeroizing::new(crate::password::random_password()?))
        .await
        .map_err(|error| SampleError::step(Step::PasswordHasher, &error))?;
    let at = world.clock.now();
    let mut new_people: Vec<Person> = Vec::new();
    for (role, count) in &rest {
        for _ in 0..*count {
            let (first, last) = loop {
                let first = random.pick(names.first()).cloned().unwrap_or_default();
                let last = random.pick(names.last()).cloned().unwrap_or_default();
                if used.insert(format!("{first} {last}")) {
                    break (first, last);
                }
            };
            let person = Person {
                id: UserId::from_random(random16(&mut random)),
                display_name: format!("{first} {last}"),
                email: format!(
                    "{}.{}@{EMAIL_DOMAIN}",
                    first.to_ascii_lowercase(),
                    last.to_ascii_lowercase()
                ),
                created_at: at,
            };
            folk.push(Folk {
                id: person.id,
                role: *role,
            });
            new_people.push(person);
        }
    }
    let first_new = world.people.len();

    // Who works where.
    let family_count = families.len();
    let mut assignments: Vec<(usize, StartingRole, Option<usize>)> = Vec::new();
    let mut team_of_group: Vec<Vec<usize>> = vec![Vec::new(); family_count.max(1)];
    for index in first_new..folk.len() {
        let role = folk[index].role;
        match role {
            StartingRole::Administrator => {
                assignments.push((index, role, None));
                everywhere.push(index);
            }
            StartingRole::Viewer => assignments.push((index, role, None)),
            StartingRole::ProjectLead => {
                let family = random.below(family_count.max(1));
                if let Some(found) = families.get(family) {
                    assignments.push((index, role, Some(found.parent)));
                    push_team(&mut places, found.parent, index);
                }
            }
            StartingRole::TeamMember => {
                if random.chance(u32::try_from(TEAM_EVERYWHERE_PERCENT).unwrap_or(0)) {
                    assignments.push((index, role, None));
                    everywhere.push(index);
                } else {
                    for _ in 0..=random.below(FAMILIES_PER_TEAM_MEMBER) {
                        let family = random.below(family_count.max(1));
                        if let Some(found) = families.get(family) {
                            team_of_group[family].push(index);
                            assignments.push((index, role, Some(found.parent)));
                            push_team(&mut places, found.parent, index);
                        }
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
                    let family = random.below(family_count.max(1));
                    if let Some(found) = families.get(family) {
                        assignments.push((index, role, Some(found.parent)));
                        push_outsider(&mut places, found.parent, index);
                    }
                }
            }
        }
    }

    // The groups: the rest of the groups of `large.groups`, each a team of one family.
    let existing_groups = storage.groups().await?;
    let group_count = usize::try_from(plan.groups)
        .unwrap_or(0)
        .saturating_sub(existing_groups.len());
    let reserved: Vec<String> = existing_groups
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
    let mut groups: Vec<(Group, Vec<usize>)> = Vec::new();
    for at_group in 0..group_count {
        let team = random
            .pick(&words.projects.teams)
            .cloned()
            .unwrap_or_default();
        let name = format!("Team {team} {}", at_group + 1);
        let id = ordered(GroupId::from_parts, at, &mut random)?;
        let members: Vec<usize> = team_of_group
            .get(at_group % family_count.max(1))
            .cloned()
            .unwrap_or_default();
        groups.push((
            Group {
                id,
                name,
                version: FIRST_VERSION,
            },
            members,
        ));
    }

    // The weights of the projects: the sample's four projects are among the biggest.
    let shares = weights(places.len(), plan.big_projects_share);
    let mut order: Vec<usize> = (0..places.len()).collect();
    let (sample, others) = order.split_at_mut(new_from);
    random.shuffle(others);
    let ranked: Vec<usize> = sample.iter().chain(others.iter()).copied().collect();
    for (rank, place) in ranked.into_iter().enumerate() {
        places[place].weight = shares.get(rank).copied().unwrap_or(1);
    }

    // Writing.
    let given_at = at;
    let mut rows: Vec<Assignment> = Vec::new();
    for (index, role, family_parent) in &assignments {
        let project = family_parent.map(|parent| places[parent].id);
        rows.push(Assignment {
            id: ordered(AssignmentId::from_parts, given_at, &mut random)?,
            person: Some(folk[*index].id),
            group: None,
            role: Some(roles.of(*role)?),
            project,
            applies_to: if project.is_some() {
                AppliesTo::ProjectAndSubProjects
            } else {
                AppliesTo::Organization
            },
            given_at,
        });
    }
    let group_role = roles.of(StartingRole::TeamMember)?;
    let mut group_rows: Vec<(Assignment, Vec<GroupMember>)> = Vec::new();
    for (at_group, (group, members)) in groups.iter().enumerate() {
        let parent = families
            .get(at_group % family_count.max(1))
            .map(|family| family.parent);
        let project = parent.map(|parent| places[parent].id);
        group_rows.push((
            Assignment {
                id: ordered(AssignmentId::from_parts, given_at, &mut random)?,
                person: None,
                group: Some(group.id),
                role: Some(group_role),
                project,
                applies_to: AppliesTo::ProjectAndSubProjects,
                given_at,
            },
            members
                .iter()
                .map(|member| GroupMember {
                    group: group.id,
                    person: folk[*member].id,
                    added_at: given_at,
                })
                .collect(),
        ));
    }
    storage
        .database()
        .record_changes(&[], async |transaction| {
            for person in &new_people {
                AuthStorage::insert_person(transaction, person, &hash, at).await?;
                bb_app::dataset_events::give_handle::<AuthStorage>(
                    transaction,
                    person,
                    at,
                    &reserved,
                )
                .await?;
            }
            for project in &new_projects {
                AuthStorage::insert_project(transaction, project).await?;
            }
            for (group, _) in &groups {
                AuthStorage::insert_group(transaction, group).await?;
            }
            for row in &rows {
                AuthStorage::insert_assignment(transaction, row).await?;
            }
            for (row, members) in &group_rows {
                AuthStorage::insert_assignment(transaction, row).await?;
                for member in members {
                    AuthStorage::insert_group_member(transaction, member).await?;
                }
            }
            Ok(())
        })
        .await?;
    everywhere.sort_unstable();
    everywhere.dedup();
    Ok(Org {
        folk,
        everywhere,
        places,
    })
}

fn push_team(places: &mut [Place], parent: usize, index: usize) {
    let family = places[parent].id;
    for place in places
        .iter_mut()
        .filter(|place| place.chain.last() == Some(&family))
    {
        if !place.team.contains(&index) {
            place.team.push(index);
        }
    }
}

fn push_outsider(places: &mut [Place], parent: usize, index: usize) {
    let family = places[parent].id;
    for place in places
        .iter_mut()
        .filter(|place| place.chain.last() == Some(&family))
    {
        if !place.outsiders.contains(&index) {
            place.outsiders.push(index);
        }
    }
}
