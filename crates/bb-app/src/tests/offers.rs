//! Where issue types and groups of labels are offered, and who looks after them (ISS-02, TAG-02,
//! SEC-03, M3-F22, M3-F122, M3-F124, ADR-0027 amendment). The rules, run against the in-memory
//! store; the same rules run on both databases through the API in `bb-server`.

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::id::{IssueId, IssueTypeId, LabelGroupId, LabelId, ProjectId};
use bb_core::offer::OfferSubject;
use bb_policy::{Authorized, StartingRole};

use super::scenarios::{a_project, token};
use super::scenarios_issues::{a_type, an_issue};
use super::workflows_rules::member;
use super::world::{Outcome, World};
use crate::access::ProjectLabel;
use crate::auth::{AppError, Started};
use crate::issues::{
    ChangeIssue, ChangeIssueType, CreateIssue, CreateIssueType, IssueTypeView, IssueTypesQuery,
    MoveIssue, change_issue, change_issue_type, create_issue, create_issue_type, issue_types,
    move_issue,
};
use crate::offers::{ChangeOffers, OfferChange, OffersQuery, change_offers, offers};
use crate::projects::{CreateProject, create_project};
use crate::tags::{
    AddLabel, ChangeLabelGroup, CreateLabel, CreateLabelGroup, IssueLabelsAndTags,
    IssueLabelsAndTagsQuery, Labels, LabelsQuery, add_label, change_label_group, create_label,
    create_label_group, issue_labels_and_tags, labels,
};

/// A change that offers or does not offer a subject in a project, by its short name.
fn at(project: &str, offered: Option<bool>) -> OfferChange {
    OfferChange {
        project: Some(project.to_owned()),
        offered,
    }
}

/// The change for the whole organization.
const fn everywhere(offered: Option<bool>) -> OfferChange {
    OfferChange {
        project: None,
        offered,
    }
}

async fn types(
    world: &World,
    who: &str,
    project: Option<&str>,
) -> Result<Vec<IssueTypeView>, AppError> {
    Ok(issue_types(
        &world.ctx(),
        IssueTypesQuery {
            session_token: who.to_owned(),
            cursor: None,
            limit: 50,
            project: project.map(str::to_owned),
        },
    )
    .await?
    .into_value()
    .items)
}

async fn version_of(world: &World, issue_type: IssueTypeId) -> Result<i64, AppError> {
    Ok(types(world, &token(world), None)
        .await?
        .iter()
        .find(|found| found.id == issue_type)
        .map(|found| found.version)
        .unwrap_or_default())
}

async fn change(
    world: &World,
    who: &str,
    issue_type: IssueTypeId,
    changes: Vec<OfferChange>,
) -> Result<(), AppError> {
    change_offers(
        &world.ctx(),
        ChangeOffers {
            session_token: who.to_owned(),
            subject: OfferSubject::IssueType(issue_type),
            version: version_of(world, issue_type).await?,
            changes,
        },
    )
    .await
    .map(|_| ())
}

fn new_issue(who: &str, project: &str, issue_type: IssueTypeId) -> CreateIssue {
    CreateIssue {
        session_token: who.to_owned(),
        project: project.to_owned(),
        issue_type,
        title: "An issue".to_owned(),
        description: None,
        status: None,
        priority: None,
        assigned_to: None,
        fields: BTreeMap::new(),
    }
}

pub(super) fn refused<T>(result: &Result<T, AppError>, code: ErrorCode) -> bool {
    matches!(result, Err(AppError::Refused(found)) if *found == code)
}

/// The type of the first start that is not *Task*: *Bug*.
pub(super) async fn another_type(world: &World) -> Result<IssueTypeId, AppError> {
    let task = a_type(world).await.map_err(AppError::Internal)?;
    types(world, &token(world), None)
        .await?
        .iter()
        .map(|found| found.id)
        .find(|id| *id != task)
        .ok_or_else(|| AppError::Internal("no other type".into()))
}

#[tokio::test]
async fn iss_02_an_issue_of_a_type_the_project_does_not_offer_is_refused() -> Outcome {
    let world = World::new().await?;
    let acme = a_project(&world, "ACME").await?;
    a_project(&world, "OTHER").await?;
    let admin = token(&world);
    let (task, bug) = (a_type(&world).await?, another_type(&world).await?);
    // Until somebody says otherwise, every project offers every type.
    create_issue(&world.ctx(), new_issue(&admin, "ACME", bug)).await?;
    // ACME is limited to *Task*: *Bug* is not used there, and OTHER still offers both.
    change(&world, &admin, bug, vec![at("ACME", Some(false))]).await?;
    let refused_here = create_issue(&world.ctx(), new_issue(&admin, "ACME", bug)).await;
    assert!(refused(&refused_here, ErrorCode::IssueTypeNotOffered));
    create_issue(&world.ctx(), new_issue(&admin, "ACME", task)).await?;
    create_issue(&world.ctx(), new_issue(&admin, "OTHER", bug)).await?;
    // The list says it for the project that is asked about, and for none without one.
    let in_acme = types(&world, &admin, Some("ACME")).await?;
    let says = |views: &[IssueTypeView], id| {
        views
            .iter()
            .find(|found| found.id == id)
            .and_then(|found| found.here.as_ref())
            .map(|here| (here.offered, here.own, here.may_change))
    };
    assert_eq!(says(&in_acme, bug), Some((false, Some(false), true)));
    assert_eq!(says(&in_acme, task), Some((true, None, true)));
    assert_eq!(
        says(&types(&world, &admin, Some("OTHER")).await?, bug),
        Some((true, None, true))
    );
    assert_eq!(says(&types(&world, &admin, None).await?, bug), None);
    // Changing the type of an issue to one the project does not use is refused as well.
    let issue = an_issue(&world, acme).await?;
    let changed = change_issue(
        &world.ctx(),
        ChangeIssue {
            session_token: admin.clone(),
            issue: issue.to_string(),
            version: 1,
            issue_type: Some(bug),
            title: None,
            status: None,
            priority: None,
            assigned_to: None,
            fields: BTreeMap::new(),
        },
    )
    .await;
    assert!(refused(&changed, ErrorCode::IssueTypeNotOffered));
    Ok(())
}

#[tokio::test]
async fn iss_02_a_sub_project_follows_its_parent_until_it_has_its_own_row() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let acme = a_project(&world, "ACME").await?;
    create_project(
        &world.ctx(),
        CreateProject {
            session_token: admin.clone(),
            parent: Some(acme),
            short_name: "SUB".to_owned(),
            name: "Sub".to_owned(),
        },
    )
    .await?;
    let bug = another_type(&world).await?;
    change(&world, &admin, bug, vec![at("ACME", Some(false))]).await?;
    let follows = create_issue(&world.ctx(), new_issue(&admin, "SUB", bug)).await;
    assert!(refused(&follows, ErrorCode::IssueTypeNotOffered));
    // The sub-project says yes for itself.
    change(&world, &admin, bug, vec![at("SUB", Some(true))]).await?;
    create_issue(&world.ctx(), new_issue(&admin, "SUB", bug)).await?;
    // Taking its row away makes it follow ACME again.
    change(&world, &admin, bug, vec![at("SUB", None)]).await?;
    let again = create_issue(&world.ctx(), new_issue(&admin, "SUB", bug)).await;
    assert!(refused(&again, ErrorCode::IssueTypeNotOffered));
    Ok(())
}

/// A person who is Project lead of the project, with the session of their sign-in.
async fn lead_of(world: &World, email: &str, project: ProjectId) -> Result<Started, AppError> {
    member(world, email, StartingRole::ProjectLead, project).await
}

#[tokio::test]
async fn sec_03_a_project_lead_manages_the_types_of_their_project_only() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let acme = a_project(&world, "ACME").await?;
    a_project(&world, "OTHER").await?;
    let lead = lead_of(&world, "lena@example.test", acme)
        .await?
        .session_token;
    let bug = another_type(&world).await?;
    // A type made for the whole organization needs the permission for the whole organization.
    let whole = create_issue_type(
        &world.ctx(),
        CreateIssueType {
            session_token: lead.clone(),
            project: None,
            name: "Question".to_owned(),
            icon: bb_core::issue::IssueIcon::CircleHelp,
        },
    )
    .await;
    assert!(refused(&whole, ErrorCode::Forbidden));
    let renamed = change_issue_type(
        &world.ctx(),
        ChangeIssueType {
            session_token: lead.clone(),
            issue_type: bug,
            version: 1,
            name: Some("Defect".to_owned()),
            icon: None,
            retired: None,
        },
    )
    .await;
    assert!(refused(&renamed, ErrorCode::Forbidden));
    // A project the lead does not know is not there for them.
    let elsewhere = create_issue_type(
        &world.ctx(),
        CreateIssueType {
            session_token: lead.clone(),
            project: Some("OTHER".to_owned()),
            name: "Question".to_owned(),
            icon: bb_core::issue::IssueIcon::CircleHelp,
        },
    )
    .await;
    assert!(refused(&elsewhere, ErrorCode::NotFound));
    // In their own project the lead makes a type and looks after it.
    let made = create_issue_type(
        &world.ctx(),
        CreateIssueType {
            session_token: lead.clone(),
            project: Some("ACME".to_owned()),
            name: "Question".to_owned(),
            icon: bb_core::issue::IssueIcon::CircleHelp,
        },
    )
    .await?
    .into_value();
    assert!(matches!(
        made.created_in,
        Some(ProjectLabel::Visible { ref short_name, .. }) if short_name == "ACME"
    ));
    assert!(made.may_manage);
    change_issue_type(
        &world.ctx(),
        ChangeIssueType {
            session_token: lead.clone(),
            issue_type: made.id,
            version: made.version,
            name: Some("Ask".to_owned()),
            icon: None,
            retired: None,
        },
    )
    .await?;
    // The administrator sees it as one they look after too; the lead's own type is not the
    // organization's: it is offered in ACME only.
    create_issue(&world.ctx(), new_issue(&lead, "ACME", made.id)).await?;
    let other = create_issue(&world.ctx(), new_issue(&admin, "OTHER", made.id)).await;
    assert!(refused(&other, ErrorCode::IssueTypeNotOffered));
    // The lead switches an organization's type off in their project, and cannot do it anywhere else.
    change(&world, &lead, bug, vec![at("ACME", Some(false))]).await?;
    let for_all = change(&world, &lead, bug, vec![everywhere(Some(false))]).await;
    assert!(refused(&for_all, ErrorCode::Forbidden));
    let for_other = change(&world, &lead, bug, vec![at("OTHER", Some(false))]).await;
    assert!(refused(&for_other, ErrorCode::NotFound));
    // A change is judged place by place and made whole or not at all.
    let both = change(
        &world,
        &lead,
        bug,
        vec![at("ACME", Some(true)), everywhere(Some(false))],
    )
    .await;
    assert!(refused(&both, ErrorCode::Forbidden));
    let still_off = create_issue(&world.ctx(), new_issue(&admin, "ACME", bug)).await;
    assert!(refused(&still_off, ErrorCode::IssueTypeNotOffered));
    // A type made in ACME takes no row above ACME, not even from the administrator.
    let above = change(&world, &admin, made.id, vec![everywhere(Some(true))]).await;
    assert!(refused(&above, ErrorCode::InvalidRequest));
    let same_place_twice = change(
        &world,
        &admin,
        bug,
        vec![at("ACME", Some(true)), at("ACME", Some(false))],
    )
    .await;
    assert!(refused(&same_place_twice, ErrorCode::InvalidRequest));
    Ok(())
}

#[tokio::test]
async fn iss_02_a_change_of_the_offers_is_made_on_the_version_that_was_looked_at() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    a_project(&world, "ACME").await?;
    let bug = another_type(&world).await?;
    let looked_at = version_of(&world, bug).await?;
    change(&world, &admin, bug, vec![at("ACME", Some(false))]).await?;
    // The change counted the version of the type up, so the old version is stopped.
    let late = change_offers(
        &world.ctx(),
        ChangeOffers {
            session_token: admin.clone(),
            subject: OfferSubject::IssueType(bug),
            version: looked_at,
            changes: vec![at("ACME", Some(true))],
        },
    )
    .await;
    assert!(matches!(late, Err(AppError::Stale(_))));
    // The rows can be read, a page at a time, by anybody who is signed in.
    let page = offers(
        &world.ctx(),
        OffersQuery {
            session_token: admin,
            subject: OfferSubject::IssueType(bug),
            cursor: None,
            limit: 50,
        },
    )
    .await?
    .into_value();
    assert_eq!(page.items.len(), 1);
    assert!(!page.items[0].offered);
    Ok(())
}

#[tokio::test]
async fn iss_02_a_type_the_target_does_not_offer_is_named_and_kept_on_a_move() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let acme = a_project(&world, "ACME").await?;
    a_project(&world, "OTHER").await?;
    let task = a_type(&world).await?;
    change(&world, &admin, task, vec![at("OTHER", Some(false))]).await?;
    let issue = an_issue(&world, acme).await?;
    let moved = move_issue(
        &world.ctx(),
        MoveIssue {
            session_token: admin.clone(),
            issue: issue.to_string(),
            version: 1,
            project: "OTHER".to_owned(),
        },
    )
    .await?
    .into_value();
    // The issue keeps the type that the project it went into does not use.
    assert_eq!(moved.issue_type.id, task);
    Ok(())
}

// ---- Groups of labels ----

async fn group_in(
    world: &World,
    who: &str,
    project: Option<&str>,
    name: &str,
) -> Result<LabelGroupId, AppError> {
    Ok(create_label_group(
        &world.ctx(),
        CreateLabelGroup {
            session_token: who.to_owned(),
            project: project.map(str::to_owned),
            name: name.to_owned(),
            pick_one: false,
        },
    )
    .await?
    .into_value()
    .id)
}

async fn all_labels(world: &World, who: &str, project: Option<&str>) -> Result<Labels, AppError> {
    Ok(labels(
        &world.ctx(),
        LabelsQuery {
            session_token: who.to_owned(),
            project: project.map(str::to_owned),
        },
    )
    .await?
    .into_value())
}

/// ACME with a sub-project SUB, a project OTHER, and a lead of ACME; the lead made the group
/// *Area* in ACME, with the label *login* in it.
struct Groups {
    world: World,
    admin: String,
    lead: String,
    acme: ProjectId,
    sub: ProjectId,
    other: ProjectId,
    area: LabelGroupId,
    login: LabelId,
}

async fn groups() -> Result<Groups, Box<dyn std::error::Error + Send + Sync>> {
    let world = World::new().await?;
    let admin = token(&world);
    let acme = a_project(&world, "ACME").await?;
    let other = a_project(&world, "OTHER").await?;
    let sub = create_project(
        &world.ctx(),
        CreateProject {
            session_token: admin.clone(),
            parent: Some(acme),
            short_name: "SUB".to_owned(),
            name: "Sub".to_owned(),
        },
    )
    .await?
    .into_value()
    .id;
    let lead = lead_of(&world, "lena@example.test", acme)
        .await?
        .session_token;
    let area = group_in(&world, &lead, Some("ACME"), "Area").await?;
    let login = create_label(
        &world.ctx(),
        CreateLabel {
            session_token: lead.clone(),
            name: "login".to_owned(),
            color: None,
            group: Some(area),
        },
    )
    .await?
    .into_value()
    .id;
    Ok(Groups {
        world,
        admin,
        lead,
        acme,
        sub,
        other,
        area,
        login,
    })
}

async fn label_on(
    world: &World,
    who: &str,
    issue: IssueId,
    label: LabelId,
) -> Result<Authorized<IssueLabelsAndTags>, AppError> {
    add_label(
        &world.ctx(),
        AddLabel {
            session_token: who.to_owned(),
            issue: issue.to_string(),
            label,
        },
    )
    .await
}

#[tokio::test]
async fn tag_02_a_label_group_of_a_project_is_offered_there_and_below() -> Outcome {
    let Groups {
        world,
        admin,
        lead,
        acme,
        sub,
        other,
        area,
        login,
    } = groups().await?;
    let (in_acme, in_sub) = (an_issue(&world, acme).await?, an_issue(&world, sub).await?);
    let in_other = an_issue(&world, other).await?;
    label_on(&world, &admin, in_acme, login).await?;
    label_on(&world, &admin, in_sub, login).await?;
    let outside = label_on(&world, &admin, in_other, login).await;
    assert!(refused(&outside, ErrorCode::LabelNotOffered));
    // The list says it for the project that is asked about.
    let says = |listed: &Labels| {
        listed
            .groups
            .iter()
            .find(|found| found.id == area)
            .and_then(|found| found.here.as_ref())
            .map(|here| here.offered)
    };
    for (project, offered) in [
        ("ACME", Some(true)),
        ("SUB", Some(true)),
        ("OTHER", Some(false)),
    ] {
        let listed = all_labels(&world, &admin, Some(project)).await?;
        assert_eq!(says(&listed), offered, "{project}");
    }
    let listed = all_labels(&world, &admin, None).await?;
    assert_eq!(says(&listed), None);
    // The sub-project can leave it off on its own, and the lead of ACME may say so.
    let version = |listed: &Labels| {
        listed
            .groups
            .iter()
            .find(|found| found.id == area)
            .map(|found| found.version)
            .unwrap_or_default()
    };
    change_offers(
        &world.ctx(),
        ChangeOffers {
            session_token: lead,
            subject: OfferSubject::LabelGroup(area),
            version: version(&listed),
            changes: vec![at("SUB", Some(false))],
        },
    )
    .await?;
    let listed = all_labels(&world, &admin, Some("SUB")).await?;
    assert_eq!(says(&listed), Some(false));
    Ok(())
}

#[tokio::test]
async fn tag_02_a_label_stays_on_an_issue_that_moves_out_of_its_project() -> Outcome {
    let Groups {
        world,
        admin,
        acme,
        login,
        ..
    } = groups().await?;
    let issue = an_issue(&world, acme).await?;
    label_on(&world, &admin, issue, login).await?;
    move_issue(
        &world.ctx(),
        MoveIssue {
            session_token: admin.clone(),
            issue: issue.to_string(),
            version: 2,
            project: "OTHER".to_owned(),
        },
    )
    .await?;
    // OTHER does not use the group, and the label is still on the issue and is shown, the way a
    // retired label is.
    let shown = issue_labels_and_tags(
        &world.ctx(),
        IssueLabelsAndTagsQuery {
            session_token: admin,
            issue: issue.to_string(),
        },
    )
    .await?
    .into_value();
    assert_eq!(shown.labels.len(), 1);
    assert_eq!(shown.labels[0].id, login);
    Ok(())
}

#[tokio::test]
async fn sec_03_a_project_lead_manages_the_label_groups_of_their_project_only() -> Outcome {
    let Groups {
        world,
        admin,
        lead,
        area,
        ..
    } = groups().await?;
    // The whole organization is not the lead's: neither a group of it nor a label in one.
    let whole = group_in(&world, &lead, None, "Platform").await;
    assert!(refused(&whole, ErrorCode::Forbidden));
    let organization_group = group_in(&world, &admin, None, "Platform").await?;
    let into_theirs = create_label(
        &world.ctx(),
        CreateLabel {
            session_token: lead.clone(),
            name: "web".to_owned(),
            color: None,
            group: Some(organization_group),
        },
    )
    .await;
    assert!(refused(&into_theirs, ErrorCode::Forbidden));
    // The lead changes their own group, and not the group of the organization.
    let rename = |session: &str, group, version| ChangeLabelGroup {
        session_token: session.to_owned(),
        group,
        version,
        name: Some("Renamed".to_owned()),
        pick_one: None,
        retired: None,
    };
    let own = all_labels(&world, &lead, None).await?;
    let version = own
        .groups
        .iter()
        .find(|found| found.id == area)
        .map(|found| found.version)
        .unwrap_or_default();
    assert!(
        own.groups
            .iter()
            .any(|found| found.id == area && found.may_manage)
    );
    change_label_group(&world.ctx(), rename(&lead, area, version)).await?;
    let theirs_not = change_label_group(&world.ctx(), rename(&lead, organization_group, 1)).await;
    assert!(refused(&theirs_not, ErrorCode::Forbidden));
    assert!(
        !own.groups
            .iter()
            .any(|found| found.id == organization_group && found.may_manage)
    );
    Ok(())
}
