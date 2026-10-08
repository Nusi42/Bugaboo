//! What follows the people (architecture §11): the groups, the roles, the projects with their
//! sub-projects and who has which role where, made through the use cases the API calls, as the
//! first administrator.
//!
//! The *plan* is data and rules: [`plan`] works out from the roster who is in which group and which
//! rows of *Who has which role* there are, and has no database in it, so its tests are quick. The
//! *making* ([`Organization`]) follows the plan step by step, one change every second of the
//! sample's clock.
//!
//! The mix of roles of the roster is given in four ways, so every kind of row is there for the
//! screens and for *Check access* to explain: for the whole organization, on a project (and its
//! sub-projects), through a group, and as *No access* on a sub-project.

use std::collections::BTreeMap;

use bb_app::access::{Permission, PolicyStore as _, RoleRecord, StartingRole};
use bb_app::people::{AddGroupMember, MakeGroup, add_group_member, make_group};
use bb_app::projects::{MakeProject, make_project};
use bb_app::roles::{
    AssignRole, CopyRole, Given, MakeRole, SetFieldLevel, SetPeopleLevel, Subject, assign_role,
    copy_role, make_role, set_level,
};
use bb_core::access::{AppliesTo, FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::id::{GroupId, ProjectId, RoleId};
use bb_core::people_kinds::PeopleKind;
use bb_core::project::ShortName;

use crate::error::{SampleError, Step};
use crate::generate::{Later, World};
use crate::layout::{Layout, OrganizationNames};
use crate::roster::{Role, SamplePerson};

/// A role the plan gives: one of the six starting roles, or a custom role of the sample (by the
/// key it has in `tools/dev/sample.json`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RoleKey {
    Starting(StartingRole),
    Custom(&'static str),
}

/// Who a row is for: a person (their place in the roster) or a group (its key).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Holder {
    Person(usize),
    Group(&'static str),
}

/// What a row gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Grant {
    Role(RoleKey),
    NoAccess,
}

/// One row of *Who has which role*.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Row {
    pub(crate) holder: Holder,
    pub(crate) grant: Grant,
    /// The short name of the project, or `None` for the whole organization.
    pub(crate) project: Option<String>,
    pub(crate) applies_to: AppliesTo,
}

/// Everything the plan says.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Plan {
    /// The people of each group (their places in the roster), in the order of [`GROUPS`].
    pub(crate) members: Vec<(&'static str, Vec<usize>)>,
    /// The rows, in the order they are given.
    pub(crate) rows: Vec<Row>,
}

/// A group, who it is made of and what it gives its members.
struct GroupRule {
    /// The key of the group in `sample.json`.
    key: &'static str,
    /// The people of this role (in the roster) are its members.
    of: Role,
    /// The role it gives.
    grants: RoleKey,
    /// The projects it gives it on (and in their sub-projects); none means the whole organization.
    on: &'static [&'static str],
}

const TEAM_MEMBER: RoleKey = RoleKey::Starting(StartingRole::TeamMember);

/// The eight groups of architecture §11.
const GROUPS: [GroupRule; 8] = [
    GroupRule {
        key: "backend",
        of: Role::TeamMember,
        grants: TEAM_MEMBER,
        on: &["WEB", "OPS"],
    },
    GroupRule {
        key: "frontend",
        of: Role::TeamMember,
        grants: TEAM_MEMBER,
        on: &["WEB"],
    },
    GroupRule {
        key: "mobile",
        of: Role::TeamMember,
        grants: TEAM_MEMBER,
        on: &["APP"],
    },
    GroupRule {
        key: "qa",
        of: Role::TeamMember,
        grants: TEAM_MEMBER,
        on: &["APP", "WEB"],
    },
    GroupRule {
        key: "support",
        of: Role::TeamMember,
        grants: RoleKey::Custom(SUPPORT_AGENT),
        on: &["OPS"],
    },
    GroupRule {
        key: "docs",
        of: Role::TeamMember,
        grants: TEAM_MEMBER,
        on: &["DOC"],
    },
    GroupRule {
        key: "management",
        of: Role::Viewer,
        grants: RoleKey::Starting(StartingRole::Viewer),
        on: &[],
    },
    GroupRule {
        key: "external",
        of: Role::Contributor,
        grants: RoleKey::Starting(StartingRole::Contributor),
        on: &["WEB"],
    },
];

/// The key of the custom role that is a Team member without *Cost center*.
pub(crate) const SUPPORT_AGENT: &str = "support-agent";
/// The key of the custom role that needs two-step sign-in.
pub(crate) const SECURITY_REVIEWER: &str = "security-reviewer";

/// How many of the Contributors are in the group *External* (and hold their role through it).
const EXTERNAL_CONTRIBUTORS: usize = 4;
/// Of every five Team members, this many (the last ones) hold their role only through a group.
const TEAM_THROUGH_GROUP: usize = 2;
/// The project a code-app holder reviews.
const REVIEWED_PROJECT: &str = "OPS";
/// The sub-project the person named in the demos may not enter.
const NO_ACCESS_ON: &str = "AND";
/// The projects where the two Guests have their role, in the order of the Guests.
const GUEST_PROJECTS: [&str; 2] = ["WEB", "APP"];
/// The projects where the Contributors who are not in a group have their role: two each.
const CONTRIBUTOR_PROJECTS: [&str; 2] = ["APP", "OPS"];

/// The plan while it is made: the rows so far and who is in which group.
struct Builder<'a> {
    people: &'a [SamplePerson],
    rows: Vec<Row>,
    members: BTreeMap<&'static str, Vec<usize>>,
}

impl Builder<'_> {
    /// The people of a role in the order of the roster, except that the people the demos name come
    /// first (Ana is the first Contributor, Ben the first Team member, whatever the seed).
    fn of(&self, role: Role) -> Vec<usize> {
        let mut found: Vec<usize> = self
            .people
            .iter()
            .enumerate()
            .filter(|(_, person)| person.role == role)
            .map(|(at, _)| at)
            .collect();
        found.sort_by_key(|at| !self.people[*at].is_fixed());
        found
    }

    /// A row of a person who holds a starting role: for the whole organization, or on a project.
    fn direct(&mut self, holder: usize, role: StartingRole, project: Option<&str>) {
        self.rows.push(Row {
            holder: Holder::Person(holder),
            grant: Grant::Role(RoleKey::Starting(role)),
            project: project.map(str::to_owned),
            applies_to: if project.is_some() {
                AppliesTo::ProjectAndSubProjects
            } else {
                AppliesTo::Organization
            },
        });
    }

    fn join(&mut self, group: &'static str, person: usize) {
        self.members.entry(group).or_default().push(person);
    }

    /// The second Administrator, and a Project lead for each of the projects, in order.
    fn administrators_and_leads(&mut self, names: &OrganizationNames) {
        for at in self.of(Role::Administrator).into_iter().skip(1) {
            self.direct(at, StartingRole::Administrator, None);
        }
        for (lead, project) in self.of(Role::ProjectLead).into_iter().zip(&names.projects) {
            self.direct(lead, StartingRole::ProjectLead, Some(&project.short_name));
        }
    }

    /// Team members: most for the whole organization, the last two of every five only through the
    /// groups that give the role. All of them are in a group.
    fn team_members(&mut self) {
        let giving: Vec<&GroupRule> = GROUPS
            .iter()
            .filter(|rule| rule.of == Role::TeamMember && rule.grants == TEAM_MEMBER)
            .collect();
        let teams: Vec<&GroupRule> = GROUPS
            .iter()
            .filter(|rule| rule.of == Role::TeamMember)
            .collect();
        let mut through_group = 0_usize;
        for (at, person) in self.of(Role::TeamMember).into_iter().enumerate() {
            let only_through_group = at % 5 >= 5 - TEAM_THROUGH_GROUP;
            let giver = giving.get(through_group % giving.len().max(1));
            if let (true, Some(giver)) = (only_through_group, giver) {
                through_group += 1;
                self.join(giver.key, person);
                continue;
            }
            self.direct(person, StartingRole::TeamMember, None);
            if let Some(rule) = teams.get(at % teams.len().max(1)) {
                self.join(rule.key, person);
            }
            // The person the demos name may not enter one sub-project, whatever else they hold.
            if self.people[person].is_fixed() {
                self.rows.push(Row {
                    holder: Holder::Person(person),
                    grant: Grant::NoAccess,
                    project: Some(NO_ACCESS_ON.to_owned()),
                    applies_to: AppliesTo::ProjectAndSubProjects,
                });
            }
        }
    }

    /// Contributors: the first ones (Ana is the first) in the group *External*, the others on a
    /// project each. Viewers: all through the group *Management*. Guests: a project each.
    fn the_others(&mut self) {
        for (at, person) in self.of(Role::Contributor).into_iter().enumerate() {
            if at < EXTERNAL_CONTRIBUTORS {
                if let Some(rule) = GROUPS.iter().find(|rule| rule.of == Role::Contributor) {
                    self.join(rule.key, person);
                }
            } else {
                let project = CONTRIBUTOR_PROJECTS
                    .get((at - EXTERNAL_CONTRIBUTORS) / CONTRIBUTOR_PROJECTS.len())
                    .copied();
                self.direct(person, StartingRole::Contributor, project);
            }
        }
        for person in self.of(Role::Viewer) {
            if let Some(rule) = GROUPS.iter().find(|rule| rule.of == Role::Viewer) {
                self.join(rule.key, person);
            }
        }
        for (person, project) in self.of(Role::Guest).into_iter().zip(GUEST_PROJECTS) {
            self.direct(person, StartingRole::Guest, Some(project));
        }
    }

    /// The people with a code app also hold the role that needs two-step sign-in, and each group
    /// gives its role.
    fn reviewers_and_groups(&mut self) {
        for (at, _) in self
            .people
            .iter()
            .enumerate()
            .filter(|(_, person)| person.code_app)
        {
            self.rows.push(Row {
                holder: Holder::Person(at),
                grant: Grant::Role(RoleKey::Custom(SECURITY_REVIEWER)),
                project: Some(REVIEWED_PROJECT.to_owned()),
                applies_to: AppliesTo::Project,
            });
        }
        for rule in &GROUPS {
            if rule.on.is_empty() {
                self.rows.push(Row {
                    holder: Holder::Group(rule.key),
                    grant: Grant::Role(rule.grants),
                    project: None,
                    applies_to: AppliesTo::Organization,
                });
            }
            self.rows.extend(rule.on.iter().map(|project| Row {
                holder: Holder::Group(rule.key),
                grant: Grant::Role(rule.grants),
                project: Some((*project).to_owned()),
                applies_to: AppliesTo::ProjectAndSubProjects,
            }));
        }
    }
}

/// Works out the groups and the rows from the roster.
pub(crate) fn plan(people: &[SamplePerson], names: &OrganizationNames) -> Plan {
    let mut builder = Builder {
        people,
        rows: Vec::new(),
        members: BTreeMap::new(),
    };
    builder.administrators_and_leads(names);
    builder.team_members();
    builder.the_others();
    builder.reviewers_and_groups();
    let Builder {
        rows, mut members, ..
    } = builder;
    Plan {
        members: GROUPS
            .iter()
            .map(|rule| (rule.key, members.remove(rule.key).unwrap_or_default()))
            .collect(),
        rows,
    }
}

/// Makes the groups, roles, projects and rows of the plan.
#[derive(Clone, Copy, Debug, Default)]
pub struct Organization;

impl Later for Organization {
    async fn after_people(&mut self, world: &World<'_>) -> Result<(), SampleError> {
        let names = Layout::bundled()?.organization;
        let people: Vec<SamplePerson> = world
            .people
            .iter()
            .map(|made| made.person.clone())
            .collect();
        let plan = plan(&people, &names);
        let making = Making { world };

        for rule in &GROUPS {
            making.group(names.group(rule.key)?).await?;
        }
        making.custom_roles(&names).await?;
        for project in &names.projects {
            making
                .project(&project.short_name, &project.name, None)
                .await?;
            for sub in &project.sub_projects {
                making
                    .project(&sub.short_name, &sub.name, Some(&project.short_name))
                    .await?;
            }
        }
        for (group, members) in &plan.members {
            for member in members {
                making.member(names.group(group)?, *member).await?;
            }
        }
        for row in &plan.rows {
            making.row(&names, row).await?;
        }
        making.what_roles_see().await
    }
}

/// The administrator's hands: every change is a use case, a second after the one before.
struct Making<'a, 'w> {
    world: &'a World<'w>,
}

impl Making<'_, '_> {
    /// A new moment and the administrator's session for the next change.
    fn next(&self) -> String {
        self.world.tick();
        self.world.administrator_session.to_owned()
    }

    async fn group(&self, name: &str) -> Result<(), SampleError> {
        make_group(
            &self.world.services.ctx(self.world.request()),
            MakeGroup {
                session_token: self.next(),
                name: name.to_owned(),
            },
        )
        .await
        .map(|_| ())
        .map_err(|error| SampleError::step(Step::Group, &error))
    }

    async fn member(&self, group: &str, person: usize) -> Result<(), SampleError> {
        let (group, person) = (
            self.group_id(group).await?,
            self.world
                .people
                .get(person)
                .map(|made| made.id)
                .ok_or(SampleError::PersonMissing)?,
        );
        add_group_member(
            &self.world.services.ctx(self.world.request()),
            AddGroupMember {
                session_token: self.next(),
                group,
                person,
            },
        )
        .await
        .map(|_| ())
        .map_err(|error| SampleError::step(Step::GroupMember, &error))
    }

    async fn project(
        &self,
        short_name: &str,
        name: &str,
        parent: Option<&str>,
    ) -> Result<(), SampleError> {
        let parent = match parent {
            Some(parent) => Some(self.project_id(parent).await?),
            None => None,
        };
        make_project(
            &self.world.services.ctx(self.world.request()),
            MakeProject {
                session_token: self.next(),
                parent,
                short_name: short_name.to_owned(),
                name: name.to_owned(),
            },
        )
        .await
        .map(|_| ())
        .map_err(|error| SampleError::step(Step::Project, &error))
    }

    /// The custom roles: a Team member without *Cost center*, and a role that needs two-step
    /// sign-in and sees little.
    async fn custom_roles(&self, names: &OrganizationNames) -> Result<(), SampleError> {
        let step = |error: &dyn std::fmt::Display| SampleError::step(Step::Role, &error);
        let team_member = self.starting_role(StartingRole::TeamMember).await?;
        let support = names.role(SUPPORT_AGENT)?;
        copy_role(
            &self.world.services.ctx(self.world.request()),
            CopyRole {
                session_token: self.next(),
                role: team_member,
                name: support.to_owned(),
            },
        )
        .await
        .map_err(|error| step(&error))?;
        let support = self.named_role(support).await?;
        self.field_level(support, None, Field::CostCenter, FieldLevel::Hidden)
            .await?;

        let reviewer = names.role(SECURITY_REVIEWER)?;
        make_role(
            &self.world.services.ctx(self.world.request()),
            MakeRole {
                session_token: self.next(),
                name: reviewer.to_owned(),
                permissions: vec![Permission::SeeProjects],
                two_step: true,
            },
        )
        .await
        .map_err(|error| step(&error))?;
        let reviewer = self.named_role(reviewer).await?;
        for field in [Field::Title, Field::Status, Field::Priority] {
            self.field_level(reviewer, None, field, FieldLevel::CanSee)
                .await?;
        }
        Ok(())
    }

    /// Mobile app: *Cost center* is hidden from Contributors, and Guests see no names of authors
    /// or mentions. Website keeps the starting rules, so the exit demo has something to change.
    async fn what_roles_see(&self) -> Result<(), SampleError> {
        let app = self.project_id("APP").await?;
        let contributor = self.starting_role(StartingRole::Contributor).await?;
        self.field_level(
            contributor,
            Some(app),
            Field::CostCenter,
            FieldLevel::Hidden,
        )
        .await?;
        let guest = self.starting_role(StartingRole::Guest).await?;
        for kind in [PeopleKind::Authors, PeopleKind::Mentions] {
            set_level(
                &self.world.services.ctx(self.world.request()),
                SetPeopleLevel {
                    session_token: self.next(),
                    role: guest,
                    project: Some(app),
                    kind,
                    level: Some(PeopleLevel::Hidden),
                },
            )
            .await
            .map_err(|error| SampleError::step(Step::Levels, &error))?;
        }
        Ok(())
    }

    async fn field_level(
        &self,
        role: RoleId,
        project: Option<ProjectId>,
        field: Field,
        level: FieldLevel,
    ) -> Result<(), SampleError> {
        set_level(
            &self.world.services.ctx(self.world.request()),
            SetFieldLevel {
                session_token: self.next(),
                role,
                project,
                field,
                level: Some(level),
            },
        )
        .await
        .map(|_| ())
        .map_err(|error| SampleError::step(Step::Levels, &error))
    }

    async fn row(&self, names: &OrganizationNames, row: &Row) -> Result<(), SampleError> {
        let subject = match &row.holder {
            Holder::Person(at) => Subject::Person(
                self.world
                    .people
                    .get(*at)
                    .map(|made| made.id)
                    .ok_or(SampleError::PersonMissing)?,
            ),
            Holder::Group(key) => Subject::Group(self.group_id(names.group(key)?).await?),
        };
        let given = match row.grant {
            Grant::NoAccess => Given::NoAccess,
            Grant::Role(RoleKey::Starting(role)) => Given::Role(self.starting_role(role).await?),
            Grant::Role(RoleKey::Custom(key)) => {
                Given::Role(self.named_role(names.role(key)?).await?)
            }
        };
        let project = match &row.project {
            Some(short_name) => Some(self.project_id(short_name).await?),
            None => None,
        };
        assign_role(
            &self.world.services.ctx(self.world.request()),
            AssignRole {
                session_token: self.next(),
                subject,
                given,
                project,
                applies_to: row.applies_to,
            },
        )
        .await
        .map(|_| ())
        .map_err(|error| SampleError::step(Step::Assignment, &error))
    }

    async fn group_id(&self, name: &str) -> Result<GroupId, SampleError> {
        self.world
            .storage
            .groups()
            .await?
            .into_iter()
            .find(|group| group.name == name)
            .map(|group| group.id)
            .ok_or_else(|| SampleError::NotMade(name.to_owned()))
    }

    async fn project_id(&self, short_name: &str) -> Result<ProjectId, SampleError> {
        let parsed =
            ShortName::new(short_name).map_err(|_| SampleError::NotMade(short_name.to_owned()))?;
        self.world
            .storage
            .project_by_short_name(&parsed)
            .await?
            .map(|project| project.id)
            .ok_or_else(|| SampleError::NotMade(short_name.to_owned()))
    }

    async fn roles(&self) -> Result<Vec<RoleRecord>, SampleError> {
        Ok(self.world.storage.roles().await?)
    }

    async fn starting_role(&self, role: StartingRole) -> Result<RoleId, SampleError> {
        self.roles()
            .await?
            .into_iter()
            .find(|record| {
                record
                    .role
                    .starting
                    .as_ref()
                    .map(bb_core::technical::TechnicalName::as_str)
                    == Some(role.code())
            })
            .map(|record| record.role.id)
            .ok_or_else(|| SampleError::NotMade(role.code().to_owned()))
    }

    async fn named_role(&self, name: &str) -> Result<RoleId, SampleError> {
        self.roles()
            .await?
            .into_iter()
            .find(|record| record.role.name == name)
            .map(|record| record.role.id)
            .ok_or_else(|| SampleError::NotMade(name.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::names::NameList;
    use crate::roster::roster;

    fn made() -> (Vec<SamplePerson>, OrganizationNames, Plan) {
        let people = roster(42, &NameList::bundled().unwrap()).unwrap();
        let names = Layout::bundled().unwrap().organization;
        let plan = plan(&people, &names);
        (people, names, plan)
    }

    #[test]
    fn the_plan_is_the_same_every_time_and_has_the_eight_groups() {
        let (_, _, first) = made();
        let (_, _, second) = made();
        assert_eq!(first, second);
        assert_eq!(
            first
                .members
                .iter()
                .map(|(key, _)| *key)
                .collect::<Vec<_>>(),
            GROUPS.iter().map(|rule| rule.key).collect::<Vec<_>>()
        );
        assert!(first.members.iter().all(|(_, people)| !people.is_empty()));
    }

    #[test]
    fn every_project_the_plan_names_is_in_sample_json() {
        let (_, names, plan) = made();
        let known: BTreeSet<&str> = names
            .projects
            .iter()
            .flat_map(|project| {
                std::iter::once(project.short_name.as_str()).chain(
                    project
                        .sub_projects
                        .iter()
                        .map(|sub| sub.short_name.as_str()),
                )
            })
            .collect();
        for row in &plan.rows {
            if let Some(project) = &row.project {
                assert!(known.contains(project.as_str()), "{project}");
            }
        }
        for rule in &GROUPS {
            assert!(names.group(rule.key).is_ok(), "{}", rule.key);
            assert!(rule.on.iter().all(|project| known.contains(project)));
            if let RoleKey::Custom(key) = rule.grants {
                assert!(names.role(key).is_ok());
            }
        }
    }

    #[test]
    fn ana_holds_her_role_only_through_the_group_external_and_ben_may_not_enter_android() {
        let (people, _, plan) = made();
        let at = |name: &str| {
            people
                .iter()
                .position(|person| person.name == name)
                .unwrap()
        };
        let (ana, ben) = (at("Ana Moreno"), at("Ben Hale"));
        let external = plan
            .members
            .iter()
            .find(|(key, _)| *key == "external")
            .unwrap();
        assert!(external.1.contains(&ana));
        assert!(
            !plan
                .rows
                .iter()
                .any(|row| row.holder == Holder::Person(ana)),
            "no row of her own"
        );
        assert!(
            plan.rows
                .iter()
                .any(|row| row.holder == Holder::Group("external")
                    && row.project.as_deref() == Some("WEB"))
        );
        assert!(plan.rows.iter().any(|row| row.holder == Holder::Person(ben)
            && row.grant == Grant::NoAccess
            && row.project.as_deref() == Some(NO_ACCESS_ON)));
        let no_access = plan.rows.iter().filter(|row| row.grant == Grant::NoAccess);
        assert_eq!(no_access.count(), 1);
    }

    #[test]
    fn everybody_but_the_first_administrator_holds_a_role_by_a_row_or_a_group() {
        let (people, _, plan) = made();
        for (at, person) in people.iter().enumerate().skip(1) {
            let by_row = plan.rows.iter().any(|row| row.holder == Holder::Person(at));
            let in_group = plan
                .members
                .iter()
                .any(|(_, members)| members.contains(&at));
            assert!(by_row || in_group, "{} holds nothing", person.name);
        }
    }

    #[test]
    fn a_fifth_of_the_team_members_hold_the_role_through_a_group_only() {
        let (people, _, plan) = made();
        let through_group = plan
            .members
            .iter()
            .flat_map(|(_, members)| members)
            .filter(|at| people[**at].role == Role::TeamMember)
            .filter(|at| {
                !plan
                    .rows
                    .iter()
                    .any(|row| row.holder == Holder::Person(**at))
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(through_group.len(), 12);
    }
}
