//! A page of notifications over many projects (NTF-01, SEC-06): the rights of the reader are read
//! for all the projects of the page together, and what they decide must be what one project at a
//! time would decide.

use bb_core::access::{AppliesTo, FieldLevel};
use bb_core::fields::Field;
use bb_core::id::{AssignmentId, IssueId, ProjectId, RoleId};
use bb_core::issue::Priority;
use bb_core::notification::NotificationKind;
use bb_policy::Permission;

use super::notifications::{Crew, levels};
use super::scenarios::{a_project, text, token};
use super::scenarios_issues::an_issue;
use super::world::Outcome;
use crate::auth::Started;
use crate::follow::{FollowIssue, follow_issue};
use crate::issues::{ChangeIssue, change_issue};
use crate::notifications::{NotificationWhat, UnreadQuery, unread_notifications};
use crate::people::{AddGroupMember, CreateGroup, add_group_member, create_group};
use crate::projects::{CreateProject, create_project};
use crate::roles::{
    AssignRole, CreateRole, Given, RemoveAssignment, SetFieldLevel, Subject, assign_role,
    create_role, remove_assignment, set_level,
};

/// Gives `subject` the role at the project and what is inside it, or takes everything away.
async fn assign(
    crew: &Crew,
    subject: Subject,
    given: Given,
    project: ProjectId,
) -> Result<AssignmentId, String> {
    assign_role(
        &crew.world.ctx(),
        AssignRole {
            session_token: token(&crew.world),
            subject,
            given,
            project: Some(project),
            applies_to: AppliesTo::ProjectAndSubProjects,
        },
    )
    .await
    .map(|made| made.into_value().id)
    .map_err(text)
}

/// Bob changes the title of the issue, and its priority too when `priority` is given.
async fn bob_changes(
    crew: &Crew,
    issue: IssueId,
    title: &str,
    priority: Option<Priority>,
) -> Result<(), String> {
    change_issue(
        &crew.world.ctx(),
        ChangeIssue {
            session_token: crew.bob.session_token.clone(),
            issue: issue.to_string(),
            version: 1,
            issue_type: None,
            title: Some(title.to_owned()),
            status: None,
            priority,
            assigned_to: None,
            fields: std::collections::BTreeMap::new(),
        },
    )
    .await
    .map_err(text)?;
    crew.world.clock.advance(1);
    Ok(())
}

async fn unread_of(crew: &Crew, reader: &Started) -> Result<usize, String> {
    Ok(unread_notifications(
        &crew.world.ctx(),
        UnreadQuery {
            session_token: reader.session_token.clone(),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .unread)
}

/// Who and what the test below reads: Dave's role, the projects B and C, their issues, and the
/// row that gives Dave his role in B.
struct Three {
    reader: RoleId,
    dave: Started,
    c: ProjectId,
    dave_in_b: AssignmentId,
    issue_b: IssueId,
    issue_c: IssueId,
}

/// Dave reads three projects: A (the project of the crew) through a group's role, B through his
/// own, and C inside A; he follows an issue in each.
async fn dave_reads_three_projects(crew: &Crew) -> Result<Three, String> {
    let ctx = || crew.world.ctx();
    let ada = token(&crew.world);
    let reader = create_role(
        &ctx(),
        CreateRole {
            session_token: ada.clone(),
            name: "Reader".to_owned(),
            permissions: vec![Permission::SeeProjects, Permission::SeeIssues],
            two_step: false,
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id;
    levels(
        &crew.world,
        reader,
        &[Field::Title, Field::Status, Field::Priority],
        FieldLevel::CanSee,
    )
    .await?;
    let dave = crew
        .world
        .person_named("dave@example.test", "Dave Dunn")
        .await
        .map_err(text)?;
    let group = create_group(
        &ctx(),
        CreateGroup {
            session_token: ada.clone(),
            name: "Readers".to_owned(),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id;
    add_group_member(
        &ctx(),
        AddGroupMember {
            session_token: ada.clone(),
            group,
            person: dave.person.id,
        },
    )
    .await
    .map_err(text)?;
    let (a, b) = (crew.project, a_project(&crew.world, "BETA").await?);
    let c = create_project(
        &ctx(),
        CreateProject {
            session_token: ada.clone(),
            parent: Some(a),
            short_name: "CORE".to_owned(),
            name: "Inside A".to_owned(),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id;
    assign(crew, Subject::Group(group), Given::Role(reader), a).await?;
    let dave_in_b = assign(
        crew,
        Subject::Person(dave.person.id),
        Given::Role(reader),
        b,
    )
    .await?;
    let (issue_b, issue_c) = (
        an_issue(&crew.world, b).await?,
        an_issue(&crew.world, c).await?,
    );
    for issue in [crew.issue, issue_b, issue_c] {
        follow_issue(
            &ctx(),
            FollowIssue {
                session_token: dave.session_token.clone(),
                issue: issue.to_string(),
            },
        )
        .await
        .map_err(text)?;
    }
    Ok(Three {
        reader,
        dave,
        c,
        dave_in_b,
        issue_b,
        issue_c,
    })
}

/// The role in B is taken from Dave, and his role hides the priority inside C.
async fn take_away_b_and_hide_priority_in_c(crew: &Crew, three: &Three) -> Result<(), String> {
    let ctx = || crew.world.ctx();
    remove_assignment(
        &ctx(),
        RemoveAssignment {
            session_token: token(&crew.world),
            assignment: three.dave_in_b,
        },
    )
    .await
    .map_err(text)?;
    set_level(
        &ctx(),
        SetFieldLevel {
            session_token: token(&crew.world),
            role: three.reader,
            project: Some(three.c),
            issue_type: None,
            field: Field::Priority.into(),
            level: Some(FieldLevel::Hidden),
        },
    )
    .await
    .map(|_| ())
    .map_err(text)
}

/// Afterwards B is taken from him and his role hides the priority in C. His page and his count,
/// each from one request over the three projects, must show A whole, C without the priority, and
/// nothing of B.
#[tokio::test]
async fn sec_06_a_page_over_three_projects_shows_the_two_the_reader_sees() -> Outcome {
    let crew = Crew::new().await?;
    let three = dave_reads_three_projects(&crew).await?;
    let (dave, issue_b, issue_c) = (&three.dave, three.issue_b, three.issue_c);
    crew.switch_on(NotificationKind::FollowedFields).await?;
    crew.world.clock.advance(1);
    for issue in [crew.issue, issue_b, issue_c] {
        bob_changes(&crew, issue, "A new title", Some(Priority::Critical)).await?;
    }
    // The new issues of B and C told Dave of their activity too, so there are five.
    assert_eq!(crew.heard(&dave.session_token).await?.len(), 5);
    assert_eq!(unread_of(&crew, dave).await?, 5);
    take_away_b_and_hide_priority_in_c(&crew, &three).await?;
    let (before, loads) = (
        crew.world.store.leveled_reads(),
        crew.world.store.policy_loads(),
    );
    let page = crew.heard(&dave.session_token).await?;
    assert_eq!(
        crew.world.store.leveled_reads() - before,
        1,
        "the rights in the three projects of the page come from one read"
    );
    assert_eq!(
        crew.world.store.policy_loads(),
        loads + 1,
        "only the rights for the organization are loaded on their own, none for a project"
    );

    let by_issue = |wanted: IssueId| {
        page.iter()
            .filter(|item| item.kind == NotificationKind::FollowedFields)
            .find(|item| item.issue.as_ref().is_some_and(|line| line.id == wanted))
    };
    assert!(
        page.iter()
            .all(|item| item.issue.as_ref().is_none_or(|line| line.id != issue_b)),
        "B is not a project Dave sees"
    );
    let fields_of = |wanted: IssueId| -> Result<Vec<&str>, String> {
        let Some(NotificationWhat::Changes { fields }) = by_issue(wanted).map(|item| &item.what)
        else {
            return Err("no changes".into());
        };
        Ok(fields.iter().map(|each| each.field.as_str()).collect())
    };
    let in_a = fields_of(crew.issue)?;
    assert!(
        in_a.contains(&"title") && in_a.contains(&"priority"),
        "{in_a:?}"
    );
    assert_eq!(
        fields_of(issue_c)?,
        ["title"],
        "the priority is hidden in C"
    );
    assert_eq!(page.len(), 3, "A, and C with its activity");
    assert_eq!(
        unread_of(&crew, dave).await?,
        page.len(),
        "the count is the length of the page"
    );
    Ok(())
}
