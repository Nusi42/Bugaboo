//! A small world for the tests of the engine: a project tree, roles, and the rows of one person.
//!
//! ```text
//! ACME ─ WEB ─ API        OTHER
//! ```

use std::collections::BTreeSet;

use bb_core::access::{AppliesTo, FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::id::{AssignmentId, GroupId, ProjectId, RoleId, SequentialIds, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{
    Evaluation, FieldRule, Inputs, PeopleRule, Permission, Place, RoleData, Row, Who, evaluate,
};

pub(crate) struct World {
    pub(crate) person: UserId,
    pub(crate) acme: ProjectId,
    pub(crate) web: ProjectId,
    pub(crate) api: ProjectId,
    pub(crate) other: ProjectId,
    pub(crate) roles: Vec<RoleData>,
    pub(crate) rows: Vec<Row>,
    pub(crate) field_rules: Vec<FieldRule>,
    pub(crate) people_rules: Vec<PeopleRule>,
    ids: SequentialIds,
}

impl World {
    pub(crate) fn new() -> Self {
        let mut ids = SequentialIds::default();
        Self {
            person: UserId::generate(&mut ids),
            acme: ProjectId::generate(&mut ids).unwrap(),
            web: ProjectId::generate(&mut ids).unwrap(),
            api: ProjectId::generate(&mut ids).unwrap(),
            other: ProjectId::generate(&mut ids).unwrap(),
            roles: Vec::new(),
            rows: Vec::new(),
            field_rules: Vec::new(),
            people_rules: Vec::new(),
            ids,
        }
    }

    pub(crate) fn group(&mut self) -> GroupId {
        GroupId::generate(&mut self.ids).unwrap()
    }

    /// The projects from the top down to `project`.
    pub(crate) fn chain(&self, project: ProjectId) -> Vec<ProjectId> {
        if project == self.acme {
            vec![self.acme]
        } else if project == self.web {
            vec![self.acme, self.web]
        } else if project == self.api {
            vec![self.acme, self.web, self.api]
        } else {
            vec![project]
        }
    }

    /// A new role; the order of roles is the order they are made in.
    pub(crate) fn role(&mut self, name: &str, permissions: &[Permission]) -> RoleId {
        let id = RoleId::generate(&mut self.ids).unwrap();
        self.roles.push(RoleData {
            id,
            name: name.to_owned(),
            position: i64::try_from(self.roles.len()).unwrap(),
            permissions: permissions.iter().copied().collect::<BTreeSet<_>>(),
            two_step: false,
        });
        id
    }

    fn row(
        &mut self,
        who: Who,
        role: Option<RoleId>,
        project: Option<ProjectId>,
        applies_to: AppliesTo,
    ) -> AssignmentId {
        let id = AssignmentId::generate(&mut self.ids).unwrap();
        self.rows.push(Row {
            id,
            who,
            role,
            project,
            applies_to,
        });
        id
    }

    /// The person has the role for the whole organization.
    pub(crate) fn give_everywhere(&mut self, role: RoleId) -> AssignmentId {
        self.row(Who::Person, Some(role), None, AppliesTo::Organization)
    }

    /// The person has the role on the project and its sub-projects.
    pub(crate) fn give(&mut self, role: RoleId, project: ProjectId) -> AssignmentId {
        self.row(
            Who::Person,
            Some(role),
            Some(project),
            AppliesTo::ProjectAndSubProjects,
        )
    }

    /// The person has the role only on the project.
    pub(crate) fn give_here_only(&mut self, role: RoleId, project: ProjectId) -> AssignmentId {
        self.row(Who::Person, Some(role), Some(project), AppliesTo::Project)
    }

    /// A group of the person has the role on the project and its sub-projects.
    pub(crate) fn give_group(
        &mut self,
        group: GroupId,
        role: RoleId,
        project: ProjectId,
    ) -> AssignmentId {
        self.row(
            Who::Group(group),
            Some(role),
            Some(project),
            AppliesTo::ProjectAndSubProjects,
        )
    }

    /// *No access* for the person on the project and its sub-projects.
    pub(crate) fn no_access(&mut self, project: ProjectId) -> AssignmentId {
        self.row(
            Who::Person,
            None,
            Some(project),
            AppliesTo::ProjectAndSubProjects,
        )
    }

    /// *No access* for a group of the person on the project and its sub-projects.
    pub(crate) fn no_access_group(&mut self, group: GroupId, project: ProjectId) -> AssignmentId {
        self.row(
            Who::Group(group),
            None,
            Some(project),
            AppliesTo::ProjectAndSubProjects,
        )
    }

    pub(crate) fn field(
        &mut self,
        role: RoleId,
        project: Option<ProjectId>,
        field: Field,
        level: FieldLevel,
    ) {
        self.field_rules.push(FieldRule {
            project,
            role,
            field,
            level,
        });
    }

    pub(crate) fn people(
        &mut self,
        role: RoleId,
        project: Option<ProjectId>,
        kind: PeopleKind,
        level: PeopleLevel,
    ) {
        self.people_rules.push(PeopleRule {
            project,
            role,
            kind,
            level,
        });
    }

    pub(crate) fn inputs(&self, place: Place) -> Inputs {
        Inputs {
            person: self.person,
            place,
            chain: match place {
                Place::Organization => Vec::new(),
                Place::Project(project) => self.chain(project),
            },
            rows: self.rows.clone(),
            roles: self.roles.clone(),
            field_rules: self.field_rules.clone(),
            people_rules: self.people_rules.clone(),
        }
    }

    pub(crate) fn at(&self, project: ProjectId) -> Evaluation {
        evaluate(&self.inputs(Place::Project(project)))
    }

    pub(crate) fn at_organization(&self) -> Evaluation {
        evaluate(&self.inputs(Place::Organization))
    }
}

/// What the person may do at the project, as a sorted list of keys.
pub(crate) fn keys(evaluation: &Evaluation) -> Vec<&'static str> {
    evaluation
        .permissions()
        .iter()
        .map(|permission| permission.code())
        .collect()
}
