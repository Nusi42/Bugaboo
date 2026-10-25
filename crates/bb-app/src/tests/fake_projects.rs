//! The projects part of the in-memory store: the tree, what the engine reads for one person or
//! for everybody, and the old short names.

use bb_core::id::{ProjectId, UserId};
use bb_core::project::{Project, ProjectAlias, ShortName};
use bb_policy::{LevelRules, Policy, RoleData};

use super::fake::{FakeError, FakeStore, State, Tx};
use crate::projects::{Found, ProjectStore};

/// The projects, in the order of their IDs.
fn tree_of(state: &State) -> Vec<Project> {
    let mut projects = state.projects.clone();
    projects.sort_by_key(|project| project.id);
    projects
}

/// Everything the engine reads for `person`, or for everybody when `None` (only everybody's policy
/// holds the levels of the roles).
pub(super) fn policy_of(state: &State, person: Option<UserId>) -> Policy {
    let members: Vec<_> = state
        .members
        .iter()
        .filter(|member| person.is_none_or(|person| member.person == person))
        .cloned()
        .collect();
    let assignments: Vec<_> = state
        .assignments
        .iter()
        .filter(|row| {
            person.is_none_or(|person| {
                row.person == Some(person)
                    || row
                        .group
                        .is_some_and(|group| members.iter().any(|member| member.group == group))
            })
        })
        .cloned()
        .collect();
    let named: Vec<_> = assignments.iter().filter_map(|row| row.role).collect();
    let roles: Vec<_> = state
        .roles
        .iter()
        .filter(|record| person.is_none() || named.contains(&record.role.id))
        .map(RoleData::from)
        .collect();
    Policy {
        assignments,
        members,
        roles,
        rules: if person.is_none() {
            state.rules.clone()
        } else {
            LevelRules::default()
        },
    }
}

fn owner_of(state: &State, name: &ShortName) -> Option<ProjectId> {
    state
        .projects
        .iter()
        .find(|project| &project.short_name == name)
        .map(|project| project.id)
        .or_else(|| {
            state
                .aliases
                .iter()
                .find(|alias| &alias.short_name == name)
                .map(|alias| alias.project)
        })
}

impl ProjectStore for FakeStore {
    async fn project_tree(&self) -> Result<Vec<Project>, FakeError> {
        Ok(self.read(tree_of))
    }

    async fn policy_of(&self, person: UserId) -> Result<Policy, FakeError> {
        Ok(self.read(|state| policy_of(state, Some(person))))
    }

    async fn policy_and_tree_of(
        &self,
        person: UserId,
    ) -> Result<(Policy, Vec<Project>), FakeError> {
        Ok(self.read(|state| {
            let mut policy = policy_of(state, Some(person));
            let roles = policy.roles.iter().map(|role| role.id).collect();
            policy.rules = state.rules.of_roles(&roles);
            (policy, tree_of(state))
        }))
    }

    async fn policy_all(&self) -> Result<Policy, FakeError> {
        Ok(self.read(|state| policy_of(state, None)))
    }

    async fn project_by_address(&self, name: &ShortName) -> Result<Option<Found>, FakeError> {
        Ok(self.read(|state| {
            let owner = owner_of(state, name)?;
            let project = state
                .projects
                .iter()
                .find(|project| project.id == owner)?
                .clone();
            Some(Found {
                old_name: &project.short_name != name,
                project,
            })
        }))
    }

    async fn project_tree_in(tx: &mut Tx) -> Result<Vec<Project>, FakeError> {
        Ok(tree_of(&tx.state))
    }

    async fn policy_in(tx: &mut Tx) -> Result<Policy, FakeError> {
        Ok(policy_of(&tx.state, None))
    }

    async fn short_name_owner_in(
        tx: &mut Tx,
        name: &ShortName,
    ) -> Result<Option<ProjectId>, FakeError> {
        Ok(owner_of(&tx.state, name))
    }

    async fn update_project(
        tx: &mut Tx,
        id: ProjectId,
        name: &str,
        short_name: &ShortName,
        version: i64,
    ) -> Result<bool, FakeError> {
        let Some(project) = tx
            .state
            .projects
            .iter_mut()
            .find(|project| project.id == id && project.version == version)
        else {
            return Ok(false);
        };
        name.clone_into(&mut project.name);
        project.short_name = short_name.clone();
        project.version += 1;
        Ok(true)
    }

    async fn move_project(
        tx: &mut Tx,
        id: ProjectId,
        parent: Option<ProjectId>,
        version: i64,
    ) -> Result<bool, FakeError> {
        let Some(project) = tx
            .state
            .projects
            .iter_mut()
            .find(|project| project.id == id && project.version == version)
        else {
            return Ok(false);
        };
        project.parent = parent;
        project.version += 1;
        Ok(true)
    }

    async fn insert_project_alias(tx: &mut Tx, alias: &ProjectAlias) -> Result<(), FakeError> {
        if tx
            .state
            .aliases
            .iter()
            .any(|known| known.short_name == alias.short_name)
        {
            return Err(FakeError::Duplicate);
        }
        tx.state.aliases.push(alias.clone());
        Ok(())
    }

    async fn delete_project_alias(tx: &mut Tx, name: &ShortName) -> Result<(), FakeError> {
        tx.state.aliases.retain(|alias| &alias.short_name != name);
        Ok(())
    }
}
