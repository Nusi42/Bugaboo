//! The policy part of the in-memory store: roles, groups, rows of *Who has which role*, levels and
//! projects, and the one call that loads what the engine needs for a person at a place.

use bb_core::access::{Assignment, FieldLevel, Group, GroupMember, PeopleLevel, Role};
use bb_core::fields::Field;
use bb_core::id::{AssignmentId, GroupId, ProjectId, RoleId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::project::{Project, ShortName};
use bb_policy::guards::{Holder, OrganizationState};
use bb_policy::{FieldRule, Inputs, PeopleRule, Permission, Place, RoleData, Row, Who};
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, State, Tx};
use crate::access::{PolicyStore, RoleRecord};

/// The projects from the top down to `project`; `None` when it does not exist.
fn chain_of(state: &State, project: ProjectId) -> Option<Vec<ProjectId>> {
    let mut chain = vec![project];
    let mut current = state.projects.iter().find(|known| known.id == project)?;
    while let Some(parent) = current.parent {
        current = state.projects.iter().find(|known| known.id == parent)?;
        chain.push(parent);
    }
    chain.reverse();
    Some(chain)
}

pub(super) fn role_data(record: &RoleRecord) -> RoleData {
    RoleData {
        id: record.role.id,
        name: record.role.name.clone(),
        position: record.role.position,
        permissions: record.permissions.clone(),
        two_step: record.role.two_step,
    }
}

fn inputs_of(state: &State, person: UserId, place: Place) -> Option<Inputs> {
    let chain = match place {
        Place::Organization => Vec::new(),
        Place::Project(project) => chain_of(state, project)?,
    };
    let groups: Vec<GroupId> = state
        .members
        .iter()
        .filter(|member| member.person == person)
        .map(|member| member.group)
        .collect();
    let rows: Vec<Row> = state
        .assignments
        .iter()
        .filter(|row| {
            let ours = row.person == Some(person)
                || row.group.is_some_and(|group| groups.contains(&group));
            let here = row.project.is_none_or(|project| chain.contains(&project));
            ours && here
        })
        .filter_map(|row| {
            Some(Row {
                id: row.id,
                who: if row.person == Some(person) {
                    Who::Person
                } else {
                    Who::Group(row.group?)
                },
                role: row.role,
                project: row.project,
                applies_to: row.applies_to,
            })
        })
        .collect();
    let role_ids: Vec<RoleId> = rows.iter().filter_map(|row| row.role).collect();
    let roles: Vec<RoleData> = state
        .roles
        .iter()
        .filter(|record| role_ids.contains(&record.role.id))
        .map(role_data)
        .collect();
    let here = |project: Option<ProjectId>| project.is_none_or(|project| chain.contains(&project));
    Some(Inputs {
        person,
        place,
        chain: chain.clone(),
        rows,
        roles,
        field_rules: state
            .field_levels
            .iter()
            .filter(|(role, project, _, _)| role_ids.contains(role) && here(*project))
            .map(|(role, project, field, level)| FieldRule {
                project: *project,
                role: *role,
                field: *field,
                level: *level,
            })
            .collect(),
        people_rules: state
            .people_levels
            .iter()
            .filter(|(role, project, _, _)| role_ids.contains(role) && here(*project))
            .map(|(role, project, kind, level)| PeopleRule {
                project: *project,
                role: *role,
                kind: *kind,
                level: *level,
            })
            .collect(),
    })
}

fn organization_state_of(state: &State) -> OrganizationState {
    let rows: Vec<(Holder, RoleId)> = state
        .assignments
        .iter()
        .filter(|row| row.project.is_none())
        .filter_map(|row| {
            let role = row.role?;
            let holder = match (row.person, row.group) {
                (Some(person), _) => Holder::Person(person),
                (None, Some(group)) => Holder::Group(group),
                (None, None) => return None,
            };
            Some((holder, role))
        })
        .collect();
    OrganizationState {
        roles: state
            .roles
            .iter()
            .filter(|record| rows.iter().any(|(_, role)| *role == record.role.id))
            .map(role_data)
            .collect(),
        rows,
        members: state
            .members
            .iter()
            .map(|member| (member.group, member.person))
            .collect(),
    }
}

impl PolicyStore for FakeStore {
    async fn policy_inputs(
        &self,
        person: UserId,
        place: Place,
    ) -> Result<Option<Inputs>, FakeError> {
        let mut shared = self.lock();
        shared.policy_loads += 1;
        shared.reads += 1;
        Ok(inputs_of(&shared.state, person, place))
    }

    async fn organization_state(&self) -> Result<OrganizationState, FakeError> {
        Ok(self.read(organization_state_of))
    }

    async fn starting_roles_made(&self) -> Result<bool, FakeError> {
        Ok(self.read(|state| state.starting_roles_made))
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

    async fn role(&self, id: RoleId) -> Result<Option<RoleRecord>, FakeError> {
        Ok(self.read(|state| {
            state
                .roles
                .iter()
                .find(|record| record.role.id == id)
                .cloned()
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

    async fn assignments_on(
        &self,
        project: Option<ProjectId>,
    ) -> Result<Vec<Assignment>, FakeError> {
        Ok(self.read(|state| {
            state
                .assignments
                .iter()
                .filter(|row| row.project == project)
                .cloned()
                .collect()
        }))
    }

    async fn assignments_of_group(&self, group: GroupId) -> Result<Vec<Assignment>, FakeError> {
        Ok(self.read(|state| {
            state
                .assignments
                .iter()
                .filter(|row| row.group == Some(group))
                .cloned()
                .collect()
        }))
    }

    async fn assignment(&self, id: AssignmentId) -> Result<Option<Assignment>, FakeError> {
        Ok(self.read(|state| state.assignments.iter().find(|row| row.id == id).cloned()))
    }

    async fn project(&self, id: ProjectId) -> Result<Option<Project>, FakeError> {
        Ok(self.read(|state| {
            state
                .projects
                .iter()
                .find(|project| project.id == id)
                .cloned()
        }))
    }

    async fn project_chain(&self, id: ProjectId) -> Result<Option<Vec<ProjectId>>, FakeError> {
        Ok(self.read(|state| chain_of(state, id)))
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

    async fn field_levels(
        &self,
        role: RoleId,
        project: Option<ProjectId>,
    ) -> Result<Vec<(Field, FieldLevel)>, FakeError> {
        Ok(self.read(|state| {
            state
                .field_levels
                .iter()
                .filter(|(owner, place, _, _)| *owner == role && *place == project)
                .map(|(_, _, field, level)| (*field, *level))
                .collect()
        }))
    }

    async fn people_levels(
        &self,
        role: RoleId,
        project: Option<ProjectId>,
    ) -> Result<Vec<(PeopleKind, PeopleLevel)>, FakeError> {
        Ok(self.read(|state| {
            state
                .people_levels
                .iter()
                .filter(|(owner, place, _, _)| *owner == role && *place == project)
                .map(|(_, _, kind, level)| (*kind, *level))
                .collect()
        }))
    }

    async fn lock_policy(_tx: &mut Tx) -> Result<(), FakeError> {
        Ok(())
    }

    async fn organization_state_in(tx: &mut Tx) -> Result<OrganizationState, FakeError> {
        Ok(organization_state_of(&tx.state))
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

    async fn role_in(tx: &mut Tx, id: RoleId) -> Result<Option<RoleRecord>, FakeError> {
        Ok(tx
            .state
            .roles
            .iter()
            .find(|record| record.role.id == id)
            .cloned())
    }

    async fn sub_projects_in(tx: &mut Tx, project: ProjectId) -> Result<Vec<ProjectId>, FakeError> {
        let mut found: Vec<ProjectId> = Vec::new();
        let mut next = vec![project];
        while let Some(current) = next.pop() {
            for child in tx
                .state
                .projects
                .iter()
                .filter(|known| known.parent == Some(current))
            {
                if child.id != project && !found.contains(&child.id) {
                    found.push(child.id);
                    next.push(child.id);
                }
            }
        }
        Ok(found)
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
        for (field, level) in fields {
            tx.state.field_levels.push((role.id, None, *field, *level));
        }
        for (kind, level) in people {
            tx.state.people_levels.push((role.id, None, *kind, *level));
        }
        Ok(())
    }

    async fn update_role(
        tx: &mut Tx,
        id: RoleId,
        name: &str,
        two_step: bool,
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
            .find(|record| record.role.id == id)
        else {
            return Ok(false);
        };
        name.clone_into(&mut record.role.name);
        record.role.two_step = two_step;
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
            .field_levels
            .retain(|(role, project, _, _)| !(*role == id && project.is_none()));
        tx.state
            .people_levels
            .retain(|(role, project, _, _)| !(*role == id && project.is_none()));
        for (field, level) in fields {
            tx.state.field_levels.push((id, None, *field, *level));
        }
        for (kind, level) in people {
            tx.state.people_levels.push((id, None, *kind, *level));
        }
        Ok(())
    }

    async fn delete_role(tx: &mut Tx, id: RoleId) -> Result<bool, FakeError> {
        let before = tx.state.roles.len();
        tx.state.roles.retain(|record| record.role.id != id);
        tx.state.assignments.retain(|row| row.role != Some(id));
        tx.state.field_levels.retain(|(role, _, _, _)| *role != id);
        tx.state.people_levels.retain(|(role, _, _, _)| *role != id);
        Ok(tx.state.roles.len() < before)
    }

    async fn insert_group(tx: &mut Tx, group: &Group) -> Result<(), FakeError> {
        if tx.state.groups.iter().any(|known| known.name == group.name) {
            return Err(FakeError::Duplicate);
        }
        tx.state.groups.push(group.clone());
        Ok(())
    }

    async fn rename_group(tx: &mut Tx, id: GroupId, name: &str) -> Result<bool, FakeError> {
        if tx
            .state
            .groups
            .iter()
            .any(|known| known.name == name && known.id != id)
        {
            return Err(FakeError::Duplicate);
        }
        let Some(group) = tx.state.groups.iter_mut().find(|group| group.id == id) else {
            return Ok(false);
        };
        name.clone_into(&mut group.name);
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

    async fn put_field_level(
        tx: &mut Tx,
        role: RoleId,
        project: Option<ProjectId>,
        field: Field,
        level: Option<FieldLevel>,
    ) -> Result<(), FakeError> {
        tx.state.field_levels.retain(|(owner, place, known, _)| {
            !(*owner == role && *place == project && *known == field)
        });
        if let Some(level) = level {
            tx.state.field_levels.push((role, project, field, level));
        }
        Ok(())
    }

    async fn put_people_level(
        tx: &mut Tx,
        role: RoleId,
        project: Option<ProjectId>,
        kind: PeopleKind,
        level: Option<PeopleLevel>,
    ) -> Result<(), FakeError> {
        tx.state.people_levels.retain(|(owner, place, known, _)| {
            !(*owner == role && *place == project && *known == kind)
        });
        if let Some(level) = level {
            tx.state.people_levels.push((role, project, kind, level));
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
