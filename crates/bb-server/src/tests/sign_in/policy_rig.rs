//! A small organization for the tests of the policy engine, built through the use cases a person
//! would call: an administrator, other people without a role, roles, projects, groups and rows of
//! *Who has which role*.

use bb_app::access::Permission;
use bb_app::auth::{AppError, Started};
use bb_app::people::{AddGroupMember, MakeGroup};
use bb_app::projects::{MakeProject, ProjectView};
use bb_app::roles::{
    AccessAnswer, AssignRole, AssignmentView, CheckAccessQuery, Given, MakeRole, PermissionAnswer,
    RoleView, Subject,
};
use bb_core::access::AppliesTo;
use bb_core::id::{GroupId, ProjectId, RoleId, UserId};
use bb_testing::Failure;

use super::rig::Rig;

/// What a test body returns: any error.
pub(super) type Run<T> = Result<T, Failure>;

/// A person of the test organization: who they are and the secret of their session.
#[derive(Clone)]
pub(super) struct Member {
    pub(super) id: UserId,
    pub(super) session: String,
}

/// The test organization. The administrator made it with the setup code.
pub(super) struct Org<'a> {
    pub(super) rig: &'a Rig,
    pub(super) admin: Member,
    people: std::cell::Cell<u32>,
}

impl<'a> Org<'a> {
    /// The first start and the first administrator, Ada.
    pub(super) async fn start(rig: &'a Rig) -> Run<Org<'a>> {
        let started: Started = rig.admin("ada@example.test").await?;
        Ok(Self {
            rig,
            admin: Member {
                id: started.person.id,
                session: started.session_token,
            },
            people: std::cell::Cell::new(0),
        })
    }

    /// A new person without any role, signed in.
    pub(super) async fn person(&self) -> Run<Member> {
        let number = self.people.get() + 1;
        self.people.set(number);
        let started = self
            .rig
            .person(&self.admin.session, &format!("person{number}@example.test"))
            .await?;
        Ok(Member {
            id: started.person.id,
            session: started.session_token,
        })
    }

    /// A role made by the administrator.
    pub(super) async fn role(&self, name: &str, permissions: &[Permission]) -> Run<RoleView> {
        self.make_role(&self.admin, name, permissions)
            .await
            .map_err(Into::into)
    }

    /// A role made by somebody; the answer, or why it was refused.
    pub(super) async fn make_role(
        &self,
        by: &Member,
        name: &str,
        permissions: &[Permission],
    ) -> Result<RoleView, AppError> {
        Ok(self
            .rig
            .api()
            .make_role(
                Rig::info(),
                MakeRole {
                    session_token: by.session.clone(),
                    name: name.to_owned(),
                    permissions: permissions.to_vec(),
                    two_step: false,
                },
            )
            .await?
            .into_value())
    }

    /// A project made by the administrator.
    pub(super) async fn project(
        &self,
        parent: Option<ProjectId>,
        short_name: &str,
    ) -> Run<ProjectView> {
        Ok(self
            .rig
            .api()
            .make_project(
                Rig::info(),
                MakeProject {
                    session_token: self.admin.session.clone(),
                    parent,
                    short_name: short_name.to_owned(),
                    name: format!("Project {short_name}"),
                },
            )
            .await?
            .into_value())
    }

    /// A group made by the administrator.
    pub(super) async fn group(&self, name: &str) -> Run<GroupId> {
        Ok(self
            .rig
            .api()
            .make_group(
                Rig::info(),
                MakeGroup {
                    session_token: self.admin.session.clone(),
                    name: name.to_owned(),
                },
            )
            .await?
            .into_value()
            .id)
    }

    /// A person put into a group by the administrator.
    pub(super) async fn add_to_group(&self, group: GroupId, person: UserId) -> Run<()> {
        self.rig
            .api()
            .add_group_member(
                Rig::info(),
                AddGroupMember {
                    session_token: self.admin.session.clone(),
                    group,
                    person,
                },
            )
            .await?;
        Ok(())
    }

    /// A row of *Who has which role*, given by somebody; the answer, or why it was refused.
    pub(super) async fn assign(
        &self,
        by: &Member,
        subject: Subject,
        given: Given,
        project: Option<ProjectId>,
        applies_to: AppliesTo,
    ) -> Result<AssignmentView, AppError> {
        Ok(self
            .rig
            .api()
            .assign_role(
                Rig::info(),
                AssignRole {
                    session_token: by.session.clone(),
                    subject,
                    given,
                    project,
                    applies_to,
                },
            )
            .await?
            .into_value())
    }

    /// The role for the person on the project and its sub-projects, given by the administrator.
    pub(super) async fn give(
        &self,
        person: &Member,
        role: RoleId,
        project: ProjectId,
    ) -> Result<AssignmentView, AppError> {
        self.assign(
            &self.admin,
            Subject::Person(person.id),
            Given::Role(role),
            Some(project),
            AppliesTo::ProjectAndSubProjects,
        )
        .await
    }

    /// *No access* for the person on the project, given by the administrator.
    pub(super) async fn forbid(
        &self,
        person: &Member,
        project: ProjectId,
    ) -> Result<AssignmentView, AppError> {
        self.assign(
            &self.admin,
            Subject::Person(person.id),
            Given::NoAccess,
            Some(project),
            AppliesTo::ProjectAndSubProjects,
        )
        .await
    }

    /// The role for the person for the whole organization, given by the administrator.
    pub(super) async fn give_everywhere(
        &self,
        person: &Member,
        role: RoleId,
    ) -> Result<AssignmentView, AppError> {
        self.assign(
            &self.admin,
            Subject::Person(person.id),
            Given::Role(role),
            None,
            AppliesTo::Organization,
        )
        .await
    }

    /// *Check access* by somebody, for a person at a project (or the whole organization).
    pub(super) async fn check(
        &self,
        by: &Member,
        person: UserId,
        place: Option<ProjectId>,
    ) -> Result<AccessAnswer, AppError> {
        Ok(self
            .rig
            .api()
            .check_access(
                Rig::info(),
                CheckAccessQuery {
                    session_token: by.session.clone(),
                    person,
                    place,
                },
            )
            .await?
            .into_value())
    }

    /// Whether the person has a permission at the project, as the administrator sees it with
    /// *Check access*.
    pub(super) async fn can(
        &self,
        person: &Member,
        permission: Permission,
        place: Option<ProjectId>,
    ) -> Run<bool> {
        let answer = self.check(&self.admin, person.id, place).await?;
        Ok(decision(&answer, permission).allowed)
    }
}

/// The answer for one permission.
pub(super) fn decision(answer: &AccessAnswer, permission: Permission) -> &PermissionAnswer {
    answer
        .permissions
        .iter()
        .find(|decision| decision.permission == permission)
        .unwrap_or_else(|| unreachable!("every permission has an answer"))
}
