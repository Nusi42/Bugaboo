//! The part of the canonical dump that is about the organization: groups, roles, projects, who has
//! which role where, and the levels roles set. Everything is written with the addresses of people
//! and the names of groups, roles and projects, never an ID (IDs are random by design), and sorted,
//! so the same seed gives the same text whatever order the database answers in.

use std::collections::{BTreeMap, BTreeSet};

use bb_app::access::{PolicyStore as _, RoleRecord, StartingRole};
use bb_app::fields::FieldStore as _;
use bb_app::projects::ProjectStore as _;
use bb_core::access::{AppliesTo, Assignment, Group};
use bb_core::fields::FieldKey;
use bb_core::id::{GroupId, ProjectId, RoleId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::project::Project;
use bb_core::technical::TechnicalName;
use serde::Serialize;

use crate::error::SampleError;
use crate::generate::Sample;

/// A group and the addresses of its members.
#[derive(Debug, Serialize)]
pub(crate) struct DumpGroup {
    name: String,
    members: Vec<String>,
}

/// A role, and how many people hold it (directly or through a group, anywhere).
#[derive(Debug, Serialize)]
pub(crate) struct DumpRole {
    name: String,
    starting: Option<String>,
    two_step: bool,
    permissions: Vec<String>,
    held_by: usize,
}

/// A project and the addresses of the Project leads who count in it (their own, or from above).
#[derive(Debug, Serialize)]
pub(crate) struct DumpProject {
    short_name: String,
    name: String,
    parent: Option<String>,
    leads: Vec<String>,
}

/// A row of *Who has which role*: the address of the person or the name of the group, the role (or
/// none for *No access*), the project (or none for the whole organization) and where it counts.
#[derive(Debug, Serialize)]
pub(crate) struct DumpAssignment {
    who: String,
    role: Option<String>,
    project: Option<String>,
    applies_to: &'static str,
}

/// A level a role set for a field or a kind of people information, in a project or its own.
#[derive(Debug, Serialize)]
pub(crate) struct DumpLevel {
    role: String,
    project: Option<String>,
    /// What the level is about: `field` or `people`.
    about: &'static str,
    subject: String,
    level: &'static str,
}

/// The groups, roles, projects, rows and levels, written with names and addresses.
pub(crate) struct OrganizationDump {
    pub(crate) groups: Vec<DumpGroup>,
    pub(crate) roles: Vec<DumpRole>,
    pub(crate) projects: Vec<DumpProject>,
    pub(crate) assignments: Vec<DumpAssignment>,
    pub(crate) levels: Vec<DumpLevel>,
}

/// What the names and addresses of the dump stand for.
struct Names {
    people: BTreeMap<UserId, String>,
    groups: BTreeMap<GroupId, String>,
    roles: BTreeMap<RoleId, String>,
    projects: BTreeMap<ProjectId, String>,
}

impl Names {
    fn person(&self, id: UserId) -> String {
        self.people.get(&id).cloned().unwrap_or_default()
    }

    fn role(&self, id: Option<RoleId>) -> Option<String> {
        id.and_then(|id| self.roles.get(&id).cloned())
    }

    fn project(&self, id: Option<ProjectId>) -> Option<String> {
        id.and_then(|id| self.projects.get(&id).cloned())
    }

    /// The address of a person or the name of a group, which a row is for.
    fn who(&self, row: &Assignment) -> String {
        row.person.map_or_else(
            || {
                row.group
                    .and_then(|group| self.groups.get(&group))
                    .cloned()
                    .unwrap_or_default()
            },
            |person| self.person(person),
        )
    }
}

/// What the dump reads from the store once, and the pieces are made from.
struct Read {
    projects: Vec<Project>,
    roles: Vec<RoleRecord>,
    groups: Vec<Group>,
    /// Every row for the whole organization and on every project.
    rows: Vec<Assignment>,
    members_of: BTreeMap<GroupId, BTreeSet<UserId>>,
    names: Names,
}

impl Sample {
    /// Reads the organization from the store, as the engine sees it.
    pub(crate) async fn organization(&self) -> Result<OrganizationDump, SampleError> {
        let read = self.read_organization().await?;
        Ok(OrganizationDump {
            groups: dump_groups(&read),
            roles: dump_roles(&read),
            projects: dump_projects(&read),
            assignments: dump_rows(&read),
            levels: self.dump_levels(&read).await?,
        })
    }

    async fn read_organization(&self) -> Result<Read, SampleError> {
        let storage = &self.storage;
        let projects: Vec<Project> = storage.project_tree().await?;
        let roles = storage.roles().await?;
        let groups: Vec<Group> = storage.groups().await?;
        let names = Names {
            people: self
                .people
                .iter()
                .map(|made| (made.id, made.person.email.clone()))
                .collect(),
            groups: groups
                .iter()
                .map(|group| (group.id, group.name.clone()))
                .collect(),
            roles: roles
                .iter()
                .map(|record| (record.role.id, record.role.name.clone()))
                .collect(),
            projects: projects
                .iter()
                .map(|project| (project.id, project.short_name.as_str().to_owned()))
                .collect(),
        };
        let mut rows: Vec<Assignment> = storage.assignments_on(None).await?;
        for project in &projects {
            rows.extend(storage.assignments_on(Some(project.id)).await?);
        }
        let mut members_of = BTreeMap::new();
        for group in &groups {
            let members: BTreeSet<UserId> = storage
                .group_members(group.id)
                .await?
                .into_iter()
                .map(|member| member.person)
                .collect();
            members_of.insert(group.id, members);
        }
        Ok(Read {
            projects,
            roles,
            groups,
            rows,
            members_of,
            names,
        })
    }

    /// The levels a project set for each role, and the own levels of a custom role (those of the
    /// starting roles are the starting levels, written down in the docs).
    async fn dump_levels(&self, read: &Read) -> Result<Vec<DumpLevel>, SampleError> {
        let storage = &self.storage;
        // An own field is named by an ID in its levels, which is random by design: the dump
        // names it as people do.
        let own_fields: BTreeMap<String, String> = storage
            .field_definitions()
            .await?
            .into_iter()
            .map(|definition| (definition.field.id.to_string(), definition.field.name))
            .collect();
        let mut levels = Vec::new();
        for record in &read.roles {
            let mut places: Vec<Option<ProjectId>> = read
                .projects
                .iter()
                .map(|project| Some(project.id))
                .collect();
            if record.role.starting.is_none() {
                places.push(None);
            }
            for place in places {
                for (field, level) in storage
                    .levels::<FieldKey>(record.role.id, place, None)
                    .await?
                {
                    levels.push(DumpLevel {
                        role: record.role.name.clone(),
                        project: read.names.project(place),
                        about: "field",
                        subject: {
                            let key = field.code().into_owned();
                            own_fields.get(&key).cloned().unwrap_or(key)
                        },
                        level: level.code(),
                    });
                }
                for (kind, level) in storage
                    .levels::<PeopleKind>(record.role.id, place, None)
                    .await?
                {
                    levels.push(DumpLevel {
                        role: record.role.name.clone(),
                        project: read.names.project(place),
                        about: "people",
                        subject: PeopleKind::code(kind).to_owned(),
                        level: level.code(),
                    });
                }
            }
        }
        levels.sort_by(|a, b| {
            (&a.role, &a.project, a.about, &a.subject)
                .cmp(&(&b.role, &b.project, b.about, &b.subject))
        });
        Ok(levels)
    }
}

fn dump_groups(read: &Read) -> Vec<DumpGroup> {
    let mut groups: Vec<DumpGroup> = read
        .groups
        .iter()
        .map(|group| {
            let mut members: Vec<String> = read
                .members_of
                .get(&group.id)
                .into_iter()
                .flatten()
                .map(|person| read.names.person(*person))
                .collect();
            members.sort();
            DumpGroup {
                name: group.name.clone(),
                members,
            }
        })
        .collect();
    groups.sort_by(|a, b| a.name.cmp(&b.name));
    groups
}

/// The people who hold a role, directly or through a group, anywhere.
fn holders(read: &Read, role: RoleId) -> BTreeSet<UserId> {
    let mut held = BTreeSet::new();
    for row in read.rows.iter().filter(|row| row.role == Some(role)) {
        held.extend(row.person);
        if let Some(members) = row.group.and_then(|group| read.members_of.get(&group)) {
            held.extend(members.iter().copied());
        }
    }
    held
}

fn dump_roles(read: &Read) -> Vec<DumpRole> {
    let mut roles: Vec<(i64, DumpRole)> = read
        .roles
        .iter()
        .map(|record| {
            (
                record.role.position,
                DumpRole {
                    name: record.role.name.clone(),
                    starting: record
                        .role
                        .starting
                        .as_ref()
                        .map(|key| key.as_str().to_owned()),
                    two_step: record.role.two_step,
                    permissions: record
                        .permissions
                        .iter()
                        .map(|permission| permission.code().to_owned())
                        .collect(),
                    held_by: holders(read, record.role.id).len(),
                },
            )
        })
        .collect();
    roles.sort_by_key(|(position, role)| (*position, role.name.clone()));
    roles.into_iter().map(|(_, role)| role).collect()
}

/// The addresses of the Project leads who count in a project: their row is on the project, or on
/// one above it and reaches the sub-projects.
fn leads_of(read: &Read, project: &Project, lead: Option<RoleId>) -> Vec<String> {
    let parent_of: BTreeMap<ProjectId, Option<ProjectId>> = read
        .projects
        .iter()
        .map(|known| (known.id, known.parent))
        .collect();
    let mut leads = BTreeSet::new();
    let mut at = Some(project.id);
    while let Some(current) = at {
        for row in read.rows.iter().filter(|row| row.project == Some(current)) {
            let reaches =
                current == project.id || row.applies_to == AppliesTo::ProjectAndSubProjects;
            if reaches && lead.is_some() && row.role == lead {
                leads.extend(row.person.map(|person| read.names.person(person)));
            }
        }
        at = parent_of.get(&current).copied().flatten();
    }
    leads.into_iter().collect()
}

fn dump_projects(read: &Read) -> Vec<DumpProject> {
    let lead = read
        .roles
        .iter()
        .find(|record| {
            record.role.starting.as_ref().map(TechnicalName::as_str)
                == Some(StartingRole::ProjectLead.code())
        })
        .map(|record| record.role.id);
    let mut projects: Vec<DumpProject> = read
        .projects
        .iter()
        .map(|project| DumpProject {
            short_name: project.short_name.as_str().to_owned(),
            name: project.name.clone(),
            parent: read.names.project(project.parent),
            leads: leads_of(read, project, lead),
        })
        .collect();
    projects.sort_by(|a, b| a.short_name.cmp(&b.short_name));
    projects
}

fn dump_rows(read: &Read) -> Vec<DumpAssignment> {
    let mut rows: Vec<DumpAssignment> = read
        .rows
        .iter()
        .map(|row| DumpAssignment {
            who: read.names.who(row),
            role: read.names.role(row.role),
            project: read.names.project(row.project),
            applies_to: row.applies_to.code(),
        })
        .collect();
    rows.sort_by(|a, b| {
        (&a.who, &a.role, &a.project, a.applies_to).cmp(&(
            &b.who,
            &b.role,
            &b.project,
            b.applies_to,
        ))
    });
    rows
}
