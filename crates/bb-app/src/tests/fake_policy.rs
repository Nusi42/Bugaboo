//! The policy part of the in-memory store: roles, groups, rows of *Who has which role*, levels and
//! projects, and the one call that loads what the engine needs for a person at a place.

use bb_core::access::{Assignment, FieldLevel, Group, GroupMember, PeopleLevel, Role};
use bb_core::fields::Field;
use bb_core::id::{AssignmentId, GroupId, ProjectId, RoleId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::project::{Project, ShortName};
use bb_policy::guards::OrganizationState;
use bb_policy::{Inputs, LevelRule, LevelRules, LevelSubject, Permission, Place, Tree};
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, State, Tx};
use super::fake_projects::policy_of;
use crate::access::{PlaceLevels, PolicyStore, RoleRecord};
use crate::ports::ChangeStore as _;

/// Everything the engine needs for `person` at `place`: the very question the store answers.
fn inputs_of(state: &State, person: UserId, place: Place) -> Option<Inputs> {
    let chain = match place {
        Place::Organization => Vec::new(),
        Place::Project(project) => Tree::of(&state.projects).chain(project)?,
    };
    Some(policy_of(state, None).inputs_for(person, place, chain))
}

fn organization_state_of(state: &State) -> Result<OrganizationState, FakeError> {
    // A row for nobody is broken data, here as in the store.
    OrganizationState::of(&policy_of(state, None)).map_err(|_| FakeError::Failed)
}

impl PolicyStore for FakeStore {
    async fn policy_inputs(
        &self,
        person: UserId,
        place: Place,
    ) -> Result<Option<Inputs>, FakeError> {
        // Counted for the tests that check an evaluation is made once per request.
        {
            let mut shared = self.lock();
            if shared.policy_reads_fail {
                return Err(FakeError::Failed);
            }
            shared.policy_loads += 1;
        }
        self.read_snapshot(move |tx| Box::pin(Self::policy_inputs_in(tx, person, place)))
            .await
    }

    async fn legacy_administrators(&self) -> Result<Vec<UserId>, FakeError> {
        Ok(self.read(|state| state.legacy_administrators.clone()))
    }

    async fn roles(&self) -> Result<Vec<RoleRecord>, FakeError> {
        Ok(self.read(|state| {
            let mut roles = state.roles.clone();
            roles.sort_by_key(|record| (record.role.position, record.role.id));
            roles
        }))
    }

    async fn level_rules(
        &self,
        roles: Vec<RoleId>,
        project: Option<ProjectId>,
    ) -> Result<Option<PlaceLevels>, FakeError> {
        Ok(self.read(|state| {
            let chain = match project {
                None => Vec::new(),
                Some(project) => Tree::of(&state.projects).chain(project)?,
            };
            Some(PlaceLevels {
                rules: state.rules.at(&roles.into_iter().collect(), &chain),
                chain,
            })
        }))
    }

    async fn groups(&self) -> Result<Vec<Group>, FakeError> {
        Ok(self.read(|state| {
            let mut groups = state.groups.clone();
            groups.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
            groups
        }))
    }

    async fn group(&self, id: GroupId) -> Result<Option<Group>, FakeError> {
        Ok(self.read(|state| state.groups.iter().find(|group| group.id == id).cloned()))
    }

    async fn group_members(&self, group: GroupId) -> Result<Vec<GroupMember>, FakeError> {
        Ok(self.read(|state| {
            state
                .members
                .iter()
                .filter(|member| member.group == group)
                .cloned()
                .collect()
        }))
    }

    async fn assignments_of_person(&self, person: UserId) -> Result<Vec<Assignment>, FakeError> {
        Ok(self.read(|state| {
            state
                .assignments
                .iter()
                .filter(|row| row.person == Some(person))
                .cloned()
                .collect()
        }))
    }

    async fn groups_of(&self, person: UserId) -> Result<Vec<Group>, FakeError> {
        Ok(self.read(|state| {
            let mut groups: Vec<Group> = state
                .groups
                .iter()
                .filter(|group| {
                    state
                        .members
                        .iter()
                        .any(|member| member.group == group.id && member.person == person)
                })
                .cloned()
                .collect();
            groups.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
            groups
        }))
    }

    async fn assignment(&self, id: AssignmentId) -> Result<Option<Assignment>, FakeError> {
        Ok(self.read(|state| state.assignments.iter().find(|row| row.id == id).cloned()))
    }

    async fn project_chain(&self, id: ProjectId) -> Result<Option<Vec<ProjectId>>, FakeError> {
        Ok(self.read(|state| Tree::of(&state.projects).chain(id)))
    }

    async fn project_by_short_name(&self, name: &ShortName) -> Result<Option<Project>, FakeError> {
        Ok(self.read(|state| {
            state
                .projects
                .iter()
                .find(|project| &project.short_name == name)
                .cloned()
        }))
    }

    async fn levels<K: LevelSubject>(
        &self,
        role: RoleId,
        project: Option<ProjectId>,
    ) -> Result<Vec<(K, K::Level)>, FakeError> {
        Ok(self.read(|state| {
            state
                .rules
                .of::<K>()
                .iter()
                .filter(|rule| rule.role == role && rule.project == project)
                .map(|rule| (rule.subject, rule.level))
                .collect()
        }))
    }

    async fn lock_policy(_tx: &mut Tx) -> Result<(), FakeError> {
        Ok(())
    }

    async fn organization_state_in(tx: &mut Tx) -> Result<OrganizationState, FakeError> {
        organization_state_of(&tx.state)
    }

    async fn policy_inputs_in(
        tx: &mut Tx,
        person: UserId,
        place: Place,
    ) -> Result<Option<Inputs>, FakeError> {
        Ok(inputs_of(&tx.state, person, place))
    }

    async fn assignments_on_in(
        tx: &mut Tx,
        project: Option<ProjectId>,
    ) -> Result<Vec<Assignment>, FakeError> {
        Ok(tx
            .state
            .assignments
            .iter()
            .filter(|row| row.project == project)
            .cloned()
            .collect())
    }

    async fn assignments_of_group_in(
        tx: &mut Tx,
        group: GroupId,
    ) -> Result<Vec<Assignment>, FakeError> {
        Ok(tx
            .state
            .assignments
            .iter()
            .filter(|row| row.group == Some(group))
            .cloned()
            .collect())
    }

    async fn assignments_of_role_in(
        tx: &mut Tx,
        role: RoleId,
    ) -> Result<Vec<Assignment>, FakeError> {
        Ok(tx
            .state
            .assignments
            .iter()
            .filter(|row| row.role == Some(role))
            .cloned()
            .collect())
    }

    async fn role_in(tx: &mut Tx, id: RoleId) -> Result<Option<RoleRecord>, FakeError> {
        Ok(tx
            .state
            .roles
            .iter()
            .find(|record| record.role.id == id)
            .cloned())
    }

    async fn project_in(tx: &mut Tx, id: ProjectId) -> Result<Option<Project>, FakeError> {
        Ok(tx
            .state
            .projects
            .iter()
            .find(|project| project.id == id)
            .cloned())
    }

    async fn starting_roles_made_in(tx: &mut Tx) -> Result<bool, FakeError> {
        Ok(tx.state.starting_roles_made)
    }

    async fn mark_starting_roles_made(tx: &mut Tx, _at: Timestamp) -> Result<(), FakeError> {
        tx.state.starting_roles_made = true;
        Ok(())
    }

    async fn delete_legacy_administrator(tx: &mut Tx, person: UserId) -> Result<(), FakeError> {
        tx.state
            .legacy_administrators
            .retain(|known| *known != person);
        Ok(())
    }

    async fn insert_role(
        tx: &mut Tx,
        role: &Role,
        permissions: &[Permission],
        fields: &[(Field, FieldLevel)],
        people: &[(PeopleKind, PeopleLevel)],
    ) -> Result<(), FakeError> {
        if tx
            .state
            .roles
            .iter()
            .any(|known| known.role.name == role.name)
        {
            return Err(FakeError::Duplicate);
        }
        tx.state.roles.push(RoleRecord {
            role: role.clone(),
            permissions: permissions.iter().copied().collect(),
        });
        add_own_levels(&mut tx.state.rules, role.id, fields, people);
        Ok(())
    }

    async fn update_role(
        tx: &mut Tx,
        id: RoleId,
        name: &str,
        two_step: bool,
        version: i64,
    ) -> Result<bool, FakeError> {
        if tx
            .state
            .roles
            .iter()
            .any(|known| known.role.name == name && known.role.id != id)
        {
            return Err(FakeError::Duplicate);
        }
        let Some(record) = tx
            .state
            .roles
            .iter_mut()
            .find(|record| record.role.id == id && record.role.version == version)
        else {
            return Ok(false);
        };
        name.clone_into(&mut record.role.name);
        record.role.two_step = two_step;
        record.role.version += 1;
        Ok(true)
    }

    async fn replace_role_permissions(
        tx: &mut Tx,
        id: RoleId,
        permissions: &[Permission],
    ) -> Result<(), FakeError> {
        if let Some(record) = tx
            .state
            .roles
            .iter_mut()
            .find(|record| record.role.id == id)
        {
            record.permissions = permissions.iter().copied().collect();
        }
        Ok(())
    }

    async fn replace_role_levels(
        tx: &mut Tx,
        id: RoleId,
        fields: &[(Field, FieldLevel)],
        people: &[(PeopleKind, PeopleLevel)],
    ) -> Result<(), FakeError> {
        tx.state
            .rules
            .fields
            .retain(|rule| !(rule.role == id && rule.project.is_none()));
        tx.state
            .rules
            .people
            .retain(|rule| !(rule.role == id && rule.project.is_none()));
        add_own_levels(&mut tx.state.rules, id, fields, people);
        Ok(())
    }

    async fn delete_role(tx: &mut Tx, id: RoleId) -> Result<bool, FakeError> {
        let before = tx.state.roles.len();
        tx.state.roles.retain(|record| record.role.id != id);
        tx.state.assignments.retain(|row| row.role != Some(id));
        tx.state.rules.fields.retain(|rule| rule.role != id);
        tx.state.rules.people.retain(|rule| rule.role != id);
        Ok(tx.state.roles.len() < before)
    }

    async fn insert_group(tx: &mut Tx, group: &Group) -> Result<(), FakeError> {
        if tx.state.groups.iter().any(|known| known.name == group.name) {
            return Err(FakeError::Duplicate);
        }
        tx.state.groups.push(group.clone());
        Ok(())
    }

    async fn rename_group(
        tx: &mut Tx,
        id: GroupId,
        name: &str,
        version: i64,
    ) -> Result<bool, FakeError> {
        if tx
            .state
            .groups
            .iter()
            .any(|known| known.name == name && known.id != id)
        {
            return Err(FakeError::Duplicate);
        }
        let Some(group) = tx
            .state
            .groups
            .iter_mut()
            .find(|group| group.id == id && group.version == version)
        else {
            return Ok(false);
        };
        name.clone_into(&mut group.name);
        group.version += 1;
        Ok(true)
    }

    async fn delete_group(tx: &mut Tx, id: GroupId) -> Result<bool, FakeError> {
        let before = tx.state.groups.len();
        tx.state.groups.retain(|group| group.id != id);
        tx.state.members.retain(|member| member.group != id);
        tx.state.assignments.retain(|row| row.group != Some(id));
        Ok(tx.state.groups.len() < before)
    }

    async fn insert_group_member(tx: &mut Tx, member: &GroupMember) -> Result<(), FakeError> {
        if tx
            .state
            .members
            .iter()
            .any(|known| known.group == member.group && known.person == member.person)
        {
            return Err(FakeError::Duplicate);
        }
        tx.state.members.push(member.clone());
        Ok(())
    }

    async fn delete_group_member(
        tx: &mut Tx,
        group: GroupId,
        person: UserId,
    ) -> Result<bool, FakeError> {
        let before = tx.state.members.len();
        tx.state
            .members
            .retain(|member| !(member.group == group && member.person == person));
        Ok(tx.state.members.len() < before)
    }

    async fn insert_assignment(tx: &mut Tx, assignment: &Assignment) -> Result<(), FakeError> {
        tx.state.assignments.push(assignment.clone());
        Ok(())
    }

    async fn delete_assignment(tx: &mut Tx, id: AssignmentId) -> Result<bool, FakeError> {
        let before = tx.state.assignments.len();
        tx.state.assignments.retain(|row| row.id != id);
        Ok(tx.state.assignments.len() < before)
    }

    async fn put_level<K: LevelSubject>(
        tx: &mut Tx,
        role: RoleId,
        project: Option<ProjectId>,
        subject: K,
        level: Option<K::Level>,
    ) -> Result<(), FakeError> {
        let rules = K::rules_in_mut(&mut tx.state.rules);
        rules.retain(|rule| {
            !(rule.role == role && rule.project == project && rule.subject == subject)
        });
        if let Some(level) = level {
            rules.push(LevelRule {
                project,
                role,
                subject,
                level,
            });
        }
        Ok(())
    }

    async fn insert_project(tx: &mut Tx, project: &Project) -> Result<(), FakeError> {
        if tx
            .state
            .projects
            .iter()
            .any(|known| known.short_name == project.short_name)
        {
            return Err(FakeError::Duplicate);
        }
        tx.state.projects.push(project.clone());
        Ok(())
    }
}

/// Adds the levels a role has of its own.
fn add_own_levels(
    rules: &mut LevelRules,
    role: RoleId,
    fields: &[(Field, FieldLevel)],
    people: &[(PeopleKind, PeopleLevel)],
) {
    for (subject, level) in fields {
        rules.add(LevelRule {
            project: None,
            role,
            subject: *subject,
            level: *level,
        });
    }
    for (subject, level) in people {
        rules.add(LevelRule {
            project: None,
            role,
            subject: *subject,
            level: *level,
        });
    }
}
