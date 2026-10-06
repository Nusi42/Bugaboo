//! The example requests of the routes of people, groups, roles and *Who has which role* (WP-11).
//!
//! Every example that changes something makes the thing it changes first (a group to delete, a
//! row to take away), so each actor who is let in changes a thing of their own; and the roles and
//! rows it gives hold no permission, so a person who may give rows is never refused for holding
//! less than the role (that rule has its own tests: `sec_02_*` and `sec_08_*`).

use bb_app::people::{AddGroupMember, MakeGroup};
use bb_app::roles::{AssignRole, Given, MakeRole, Subject};
use bb_core::access::AppliesTo;
use serde_json::json;

use super::actors::Project;
use super::examples::Request;
use super::routes::Fixture;
use crate::sign_in_tests::http::Failure;
use crate::sign_in_tests::rig::Rig;

/// A group, made by the administrator of the matrix.
async fn a_group(fixture: &Fixture<'_>) -> Result<String, Failure> {
    Ok(fixture
        .rig()
        .api()
        .make_group(
            Rig::info(),
            MakeGroup {
                session_token: fixture.admin_session().to_owned(),
                name: format!("Group {}", fixture.next_number()),
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value()
        .id
        .to_string())
}

/// A role that holds no permission, made by the administrator of the matrix.
async fn an_empty_role(fixture: &Fixture<'_>) -> Result<String, Failure> {
    Ok(fixture
        .rig()
        .api()
        .make_role(
            Rig::info(),
            MakeRole {
                session_token: fixture.admin_session().to_owned(),
                name: format!("Role {}", fixture.next_number()),
                permissions: Vec::new(),
                two_step: false,
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value()
        .id
        .to_string())
}

/// A row for the whole organization: a new group has a new role, given by the administrator. The
/// ID of the row.
async fn a_row(fixture: &Fixture<'_>) -> Result<String, Failure> {
    let group = a_group(fixture).await?;
    let role = an_empty_role(fixture).await?;
    let given = fixture
        .rig()
        .api()
        .assign_role(
            Rig::info(),
            AssignRole {
                session_token: fixture.admin_session().to_owned(),
                subject: Subject::Group(group.parse().map_err(|_| "a group ID")?),
                given: Given::Role(role.parse().map_err(|_| "a role ID")?),
                project: None,
                applies_to: AppliesTo::Organization,
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    Ok(given.id.to_string())
}

/// The people and the groups.
async fn groups_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
    Ok(Some(match operation {
        "listPeople" => ("GET", "/api/v1/people?search=ada".into(), None),
        "getPerson" => (
            "GET",
            format!("/api/v1/people/{}", fixture.admin_id()),
            None,
        ),
        "listGroups" => ("GET", "/api/v1/groups".into(), None),
        "makeGroup" => (
            "POST",
            "/api/v1/groups".into(),
            Some(json!({"name": format!("Made {}", fixture.next_number())})),
        ),
        "getGroup" => (
            "GET",
            format!("/api/v1/groups/{}", a_group(fixture).await?),
            None,
        ),
        "renameGroup" => (
            "PATCH",
            format!("/api/v1/groups/{}", a_group(fixture).await?),
            Some(json!({"name": format!("Renamed {}", fixture.next_number())})),
        ),
        "deleteGroup" => (
            "DELETE",
            format!("/api/v1/groups/{}", a_group(fixture).await?),
            None,
        ),
        "listGroupMembers" => (
            "GET",
            format!("/api/v1/groups/{}/members", a_group(fixture).await?),
            None,
        ),
        "addGroupMember" => (
            "PUT",
            format!(
                "/api/v1/groups/{}/members/{}",
                a_group(fixture).await?,
                fixture.admin_id()
            ),
            None,
        ),
        "removeGroupMember" => {
            let group = a_group(fixture).await?;
            fixture
                .rig()
                .api()
                .add_group_member(
                    Rig::info(),
                    AddGroupMember {
                        session_token: fixture.admin_session().to_owned(),
                        group: group.parse().map_err(|_| "a group ID")?,
                        person: fixture.admin_id(),
                    },
                )
                .await
                .map_err(|error| error.to_string())?;
            (
                "DELETE",
                format!("/api/v1/groups/{group}/members/{}", fixture.admin_id()),
                None,
            )
        }
        "listMemberChoices" => (
            "GET",
            format!(
                "/api/v1/groups/{}/member-choices?search=a",
                a_group(fixture).await?
            ),
            None,
        ),
        _ => return Ok(None),
    }))
}

/// The roles and the permission catalogue.
async fn roles_request(fixture: &Fixture<'_>, operation: &str) -> Result<Option<Request>, Failure> {
    Ok(Some(match operation {
        "listRoles" => ("GET", "/api/v1/roles".into(), None),
        "makeRole" => (
            "POST",
            "/api/v1/roles".into(),
            Some(json!({
                "name": format!("Made {}", fixture.next_number()),
                "permissions": [],
                "two_step": false
            })),
        ),
        "getRole" => ("GET", format!("/api/v1/roles/{}", fixture.a_role()?), None),
        "changeRole" => (
            "PATCH",
            format!("/api/v1/roles/{}", an_empty_role(fixture).await?),
            Some(json!({"name": format!("Changed {}", fixture.next_number())})),
        ),
        "deleteRole" => (
            "DELETE",
            format!("/api/v1/roles/{}", an_empty_role(fixture).await?),
            None,
        ),
        "copyRole" => (
            "POST",
            format!("/api/v1/roles/{}/copy", an_empty_role(fixture).await?),
            Some(json!({"name": format!("Copy {}", fixture.next_number())})),
        ),
        "resetRole" => (
            "POST",
            format!("/api/v1/roles/{}/reset", fixture.a_role()?),
            None,
        ),
        "listPermissions" => ("GET", "/api/v1/permissions".into(), None),
        _ => return Ok(None),
    }))
}

/// *Who has which role*: the rows and the choices of the form.
async fn rows_request(fixture: &Fixture<'_>, operation: &str) -> Result<Option<Request>, Failure> {
    let target = fixture.project(Project::Target).to_string();
    Ok(Some(match operation {
        "listAssignments" => ("GET", "/api/v1/assignments".into(), None),
        "assignRole" => (
            "POST",
            "/api/v1/assignments".into(),
            Some(json!({
                "who": {"group": a_group(fixture).await?},
                "role": an_empty_role(fixture).await?
            })),
        ),
        "listProjectAssignments" => (
            "GET",
            format!("/api/v1/projects/{target}/assignments"),
            None,
        ),
        "assignProjectRole" => (
            "POST",
            format!("/api/v1/projects/{target}/assignments"),
            Some(json!({
                "who": {"group": a_group(fixture).await?},
                "role": an_empty_role(fixture).await?,
                "applies_to": "project"
            })),
        ),
        "removeAssignment" => (
            "DELETE",
            format!("/api/v1/assignments/{}", a_row(fixture).await?),
            None,
        ),
        "listAssignmentChoices" => (
            "GET",
            "/api/v1/assignment-choices?kind=person&search=a".into(),
            None,
        ),
        "listProjectAssignmentChoices" => (
            "GET",
            format!("/api/v1/projects/{target}/assignment-choices?kind=role"),
            None,
        ),
        _ => return Ok(None),
    }))
}

/// The request of an example of this family, or `None` for a route of another family.
pub(super) async fn people_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
    if let Some(found) = groups_request(fixture, operation).await? {
        return Ok(Some(found));
    }
    if let Some(found) = roles_request(fixture, operation).await? {
        return Ok(Some(found));
    }
    rows_request(fixture, operation).await
}
