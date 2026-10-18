//! Who may know an issue type or a group of labels created in a project, which names may clash,
//! and how much a place may hold (SEC-03, SEC-06, ISS-02, TAG-02, ADR-0027 amendment). The rules,
//! run against the in-memory store; the same rules run on both databases through the API in
//! `bb-server`.

use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::id::ProjectId;
use bb_core::issue::{ISSUE_TYPES_MAX, IssueIcon};
use bb_core::offer::OfferSubject;
use bb_core::own_fields::Edit;
use bb_core::tags::{LABEL_GROUPS_MAX, LABELS_MAX};
use bb_policy::StartingRole;

use super::offers::{
    Groups, all_labels, another_type, change, group_in, groups, label_on, lead_of, new_issue,
    refused, types, version_of,
};
use super::scenarios_issues::an_issue;
use super::workflows_rules::{assign, member, starting_role};
use super::world::{Outcome, PASSWORD, World};
use crate::auth::{AppError, CreateAccessKey, create_access_key};
use crate::issues::{CreateIssueType, MoveIssue, create_issue_type, move_issue};
use crate::offers::{ChangeOffers, OfferChange, OffersQuery, change_offers, offers};
use crate::roles::{CheckAccessQuery, check_access};
use crate::tags::{
    ChangeLabel, CombineLabels, CreateLabel, LabelChangePreviewQuery, change_label, combine_labels,
    create_label, label_change_preview,
};

fn at(project: &str, offered: Option<bool>) -> OfferChange {
    OfferChange {
        project: Some(project.to_owned()),
        offered,
    }
}

async fn new_type(
    world: &World,
    who: &str,
    project: Option<&str>,
    name: &str,
) -> Result<crate::issues::IssueTypeView, AppError> {
    Ok(create_issue_type(
        &world.ctx(),
        CreateIssueType {
            session_token: who.to_owned(),
            project: project.map(str::to_owned),
            name: name.to_owned(),
            icon: IssueIcon::CircleHelp,
        },
    )
    .await?
    .into_value())
}

/// A key of the administrator for one project and everything inside it.
async fn key_for(world: &World, project: ProjectId) -> Result<String, AppError> {
    Ok(create_access_key(
        &world.ctx(),
        CreateAccessKey {
            session_token: world.admin.session_token.clone(),
            password: PASSWORD.to_owned(),
            name: "For one project".to_owned(),
            choice: KeyChoice::Everything,
            permissions: Vec::new(),
            projects: vec![project],
            expires_at: None,
        },
    )
    .await?
    .into_value()
    .secret)
}

/// What the readers outside ACME are: a Guest of OTHER, the Project lead of OTHER (a sibling of
/// the project of the lead of ACME) and a key for OTHER.
struct Outsiders {
    guest: String,
    sibling_lead: String,
    key: String,
}

async fn outsiders(g: &Groups) -> Result<Outsiders, AppError> {
    Ok(Outsiders {
        guest: member(&g.world, "gina@example.test", StartingRole::Guest, g.other)
            .await?
            .session_token,
        sibling_lead: lead_of(&g.world, "sam@example.test", g.other)
            .await?
            .session_token,
        key: key_for(&g.world, g.other).await?,
    })
}

#[tokio::test]
async fn sec_03_a_subject_created_in_a_project_is_unknown_outside_it() -> Outcome {
    let g = groups().await?;
    let question = new_type(&g.world, &g.lead, Some("ACME"), "Question").await?;
    let outside = outsiders(&g).await?;
    for (who, reader) in [
        ("a guest", &outside.guest),
        ("a lead of a sibling project", &outside.sibling_lead),
        ("a key for a sibling project", &outside.key),
    ] {
        // The lists leave out the type, the group and its labels.
        let listed = types(&g.world, reader, None).await?;
        assert!(listed.iter().all(|each| each.id != question.id), "{who}");
        let all = all_labels(&g.world, reader, None).await?;
        assert!(all.groups.iter().all(|each| each.id != g.area), "{who}");
        assert!(all.labels.iter().all(|each| each.id != g.login), "{who}");
        // The rows of the offers, and a change of them, are not there for them.
        for subject in [
            OfferSubject::IssueType(question.id),
            OfferSubject::LabelGroup(g.area),
        ] {
            let rows = offers(
                &g.world.ctx(),
                OffersQuery {
                    session_token: reader.clone(),
                    subject,
                    cursor: None,
                    limit: 50,
                },
            )
            .await;
            assert!(refused(&rows, ErrorCode::NotFound), "{who}");
            let changed = change_offers(
                &g.world.ctx(),
                ChangeOffers {
                    session_token: reader.clone(),
                    subject,
                    version: 1,
                    changes: vec![at("OTHER", Some(false))],
                },
            )
            .await;
            assert!(refused(&changed, ErrorCode::NotFound), "{who}");
        }
    }
    // The people who look after the project still see all of it, and so does the administrator.
    for reader in [&g.lead, &g.admin] {
        assert!(
            types(&g.world, reader, None)
                .await?
                .iter()
                .any(|each| each.id == question.id)
        );
        let all = all_labels(&g.world, reader, None).await?;
        assert!(all.groups.iter().any(|each| each.id == g.area));
        assert!(all.labels.iter().any(|each| each.id == g.login));
    }
    Ok(())
}

#[tokio::test]
async fn sec_03_check_access_does_not_name_a_type_the_checker_may_not_know() -> Outcome {
    let g = groups().await?;
    let question = new_type(&g.world, &g.lead, Some("ACME"), "Question").await?;
    let sibling = member(
        &g.world,
        "sam@example.test",
        StartingRole::ProjectLead,
        g.other,
    )
    .await?;
    let check = |issue_type| CheckAccessQuery {
        session_token: sibling.session_token.clone(),
        person: sibling.person.id,
        place: Some(g.other),
        issue_type,
    };
    check_access(&g.world.ctx(), check(None)).await?;
    let asked = check_access(&g.world.ctx(), check(Some(question.id))).await;
    assert!(refused(&asked, ErrorCode::NotFound));
    // The same question about a type of the organization is answered.
    let bug = another_type(&g.world).await?;
    let asked = check_access(&g.world.ctx(), check(Some(bug))).await?;
    assert_eq!(asked.into_value().issue_type.map(|each| each.id), Some(bug));
    Ok(())
}

#[tokio::test]
async fn sec_03_a_subject_in_use_on_an_issue_the_reader_reads_is_known() -> Outcome {
    let g = groups().await?;
    let question = new_type(&g.world, &g.lead, Some("ACME"), "Question").await?;
    let outside = outsiders(&g).await?;
    // An issue with the type and the label leaves ACME for OTHER, where the lead of OTHER reads it.
    let issue = an_issue(&g.world, g.acme).await?;
    crate::issues::change_issue(
        &g.world.ctx(),
        crate::issues::ChangeIssue {
            session_token: g.admin.clone(),
            issue: issue.to_string(),
            version: 1,
            issue_type: Some(question.id),
            title: None,
            status: None,
            priority: None,
            assigned_to: None,
            fields: std::collections::BTreeMap::new(),
        },
    )
    .await?;
    label_on(&g.world, &g.admin, issue, g.login).await?;
    let moved = move_issue(
        &g.world.ctx(),
        MoveIssue {
            session_token: g.admin.clone(),
            issue: issue.to_string(),
            version: 3,
            project: "OTHER".to_owned(),
        },
    )
    .await;
    moved?;
    let reader = &outside.sibling_lead;
    assert!(
        types(&g.world, reader, None)
            .await?
            .iter()
            .any(|each| each.id == question.id)
    );
    let all = all_labels(&g.world, reader, None).await?;
    assert!(all.groups.iter().any(|each| each.id == g.area));
    assert!(all.labels.iter().any(|each| each.id == g.login));
    let rows = offers(
        &g.world.ctx(),
        OffersQuery {
            session_token: reader.clone(),
            subject: OfferSubject::IssueType(question.id),
            cursor: None,
            limit: 50,
        },
    )
    .await;
    assert!(rows.is_ok());
    // The guest of OTHER did not report the issue and does not read it: the type is not theirs to know.
    assert!(
        types(&g.world, &outside.guest, None)
            .await?
            .iter()
            .all(|each| each.id != question.id)
    );
    Ok(())
}

#[tokio::test]
async fn sec_03_a_name_is_unique_among_the_subjects_that_can_meet_in_a_project() -> Outcome {
    let g = groups().await?;
    let other_lead = lead_of(&g.world, "sam@example.test", g.other)
        .await?
        .session_token;
    new_type(&g.world, &g.lead, Some("ACME"), "Question").await?;
    // Beside it, in a project that shares no chain with ACME, the name is free.
    new_type(&g.world, &other_lead, Some("OTHER"), "question")
        .await
        .map_err(|e| format!("beside: {e:?}"))?;
    // In the same chain, above or below, it is taken; so is the organization's own name.
    let same = new_type(&g.world, &g.lead, Some("ACME"), "QUESTION").await;
    assert!(refused(&same, ErrorCode::NameTaken));
    let below = new_type(&g.world, &g.lead, Some("SUB"), "Question").await;
    assert!(refused(&below, ErrorCode::NameTaken));
    let bug = new_type(&g.world, &g.lead, Some("ACME"), "bug").await;
    assert!(refused(&bug, ErrorCode::NameTaken));
    // The organization's own subject is unique against every subject, in any project.
    let whole = new_type(&g.world, &g.admin, None, "Question").await;
    assert!(refused(&whole, ErrorCode::NameTaken));
    // A rename is judged the same way.
    let task = new_type(&g.world, &g.lead, Some("ACME"), "Ask")
        .await
        .map_err(|e| format!("ask: {e:?}"))?;
    let renamed = crate::issues::change_issue_type(
        &g.world.ctx(),
        crate::issues::ChangeIssueType {
            session_token: g.lead.clone(),
            issue_type: task.id,
            version: task.version,
            name: Some("question".to_owned()),
            icon: None,
            retired: None,
        },
    )
    .await;
    assert!(refused(&renamed, ErrorCode::NameTaken));
    // Groups of labels follow the same rule: "Area" is ACME's.
    let again = group_in(&g.world, &g.lead, Some("SUB"), "area").await;
    assert!(refused(&again, ErrorCode::NameTaken));
    group_in(&g.world, &other_lead, Some("OTHER"), "Area")
        .await
        .map_err(|e| format!("grp: {e:?}"))?;
    let whole = group_in(&g.world, &g.admin, None, "area").await;
    assert!(refused(&whole, ErrorCode::NameTaken));
    Ok(())
}

#[tokio::test]
async fn iss_02_a_type_and_a_group_of_labels_are_the_same_name_by_one_rule() -> Outcome {
    let g = groups().await?;
    // Spaces count as little as capitals, for a type and for a group alike.
    new_type(&g.world, &g.lead, Some("ACME"), "Wide  question").await?;
    let again = new_type(&g.world, &g.lead, Some("ACME"), "wide question").await;
    assert!(refused(&again, ErrorCode::NameTaken));
    group_in(&g.world, &g.lead, Some("ACME"), "Wide  area").await?;
    let again = group_in(&g.world, &g.lead, Some("ACME"), " wide area ").await;
    assert!(refused(&again, ErrorCode::NameTaken));
    Ok(())
}

#[tokio::test]
async fn sec_03_the_rows_of_a_subject_leave_out_a_place_the_reader_may_not_know() -> Outcome {
    let g = groups().await?;
    let bug = another_type(&g.world).await?;
    change(
        &g.world,
        &g.admin,
        bug,
        vec![at("ACME", Some(false)), at("OTHER", Some(false))],
    )
    .await?;
    let rows_for = |who: &str| OffersQuery {
        session_token: who.to_owned(),
        subject: OfferSubject::IssueType(bug),
        cursor: None,
        limit: 50,
    };
    let seen = offers(&g.world.ctx(), rows_for(&g.lead))
        .await?
        .into_value()
        .items;
    // The lead of ACME sees the row of ACME and no row, hidden or not, for OTHER.
    assert_eq!(seen.len(), 1);
    assert!(matches!(
        seen[0].place,
        Some(crate::access::ProjectLabel::Visible { ref short_name, .. }) if short_name == "ACME"
    ));
    let all = offers(&g.world.ctx(), rows_for(&g.admin))
        .await?
        .into_value()
        .items;
    assert_eq!(all.len(), 2);
    // The answer to a change leaves them out as well.
    let answered = change_offers(
        &g.world.ctx(),
        ChangeOffers {
            session_token: g.lead.clone(),
            subject: OfferSubject::IssueType(bug),
            version: version_of(&g.world, bug).await?,
            changes: vec![at("ACME", Some(true))],
        },
    )
    .await?
    .into_value();
    assert_eq!(answered.rows.len(), 1);
    Ok(())
}

#[tokio::test]
async fn iss_02_each_place_has_its_own_limit_of_issue_types() -> Outcome {
    let g = groups().await?;
    let world = &g.world;
    let starting = types(world, &g.admin, None).await?.len();
    for number in 0..ISSUE_TYPES_MAX {
        new_type(world, &g.lead, Some("ACME"), &format!("Type {number}")).await?;
    }
    let full = new_type(world, &g.lead, Some("ACME"), "One too many").await;
    assert!(refused(&full, ErrorCode::LimitReached));
    // Another place, and the organization, are not full.
    new_type(world, &g.admin, Some("OTHER"), "Elsewhere").await?;
    new_type(world, &g.admin, None, "Everywhere").await?;
    assert!(starting < ISSUE_TYPES_MAX);
    Ok(())
}

#[tokio::test]
async fn tag_02_each_place_has_its_own_limit_of_groups_and_of_labels() -> Outcome {
    let g = groups().await?;
    let world = &g.world;
    for number in 1..LABEL_GROUPS_MAX {
        group_in(world, &g.lead, Some("ACME"), &format!("Group {number}")).await?;
    }
    let full = group_in(world, &g.lead, Some("ACME"), "One too many").await;
    assert!(refused(&full, ErrorCode::LimitReached));
    group_in(world, &g.admin, Some("OTHER"), "Elsewhere").await?;
    let whole = group_in(world, &g.admin, None, "Everywhere").await?;
    // The labels of the groups of ACME are counted for ACME.
    let label = |name: String, group| CreateLabel {
        session_token: g.lead.clone(),
        name,
        color: None,
        group: Some(group),
    };
    for number in 1..LABELS_MAX {
        create_label(&world.ctx(), label(format!("label {number}"), g.area)).await?;
    }
    let full = create_label(&world.ctx(), label("one too many".to_owned(), g.area)).await;
    assert!(refused(&full, ErrorCode::LimitReached));
    // The organization's group takes labels of its own.
    create_label(
        &world.ctx(),
        CreateLabel {
            session_token: g.admin.clone(),
            name: "everywhere".to_owned(),
            color: None,
            group: Some(whole),
        },
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn sec_03_a_label_of_a_project_is_not_rewritten_in_projects_its_lead_does_not_manage()
-> Outcome {
    let g = groups().await?;
    let ctx = g.world.ctx();
    let issue = an_issue(&g.world, g.acme).await?;
    label_on(&g.world, &g.admin, issue, g.login).await?;
    let other = create_label(
        &ctx,
        CreateLabel {
            session_token: g.lead.clone(),
            name: "signup".to_owned(),
            color: None,
            group: Some(g.area),
        },
    )
    .await?
    .into_value();
    let rename = |who: &str, version, name: &str| ChangeLabel {
        session_token: who.to_owned(),
        label: g.login,
        version,
        name: Some(name.to_owned()),
        color: None,
        group: Edit::Keep,
        retired: None,
    };
    // While every issue with the label is in ACME, the lead of ACME may rename it.
    let renamed = change_label(&ctx, rename(&g.lead, 1, "sign-in"))
        .await?
        .into_value();
    // The issue leaves for OTHER, and the label stays on it: the lead of ACME looks after neither.
    move_issue(
        &ctx,
        MoveIssue {
            session_token: g.admin.clone(),
            issue: issue.to_string(),
            version: 3,
            project: "OTHER".to_owned(),
        },
    )
    .await?;
    let again = change_label(&ctx, rename(&g.lead, renamed.version, "log in")).await;
    assert!(refused(&again, ErrorCode::Forbidden));
    let preview = label_change_preview(
        &ctx,
        LabelChangePreviewQuery {
            session_token: g.lead.clone(),
            label: g.login,
            name: Some("log in".to_owned()),
            into: None,
        },
    )
    .await;
    assert!(refused(&preview, ErrorCode::Forbidden));
    let combined = combine_labels(
        &ctx,
        CombineLabels {
            session_token: g.lead.clone(),
            label: g.login,
            version: renamed.version,
            into: other.id,
            into_version: other.version,
        },
    )
    .await;
    assert!(refused(&combined, ErrorCode::Forbidden));
    // Whoever holds the permission for the whole organization is not held back.
    change_label(&ctx, rename(&g.admin, renamed.version, "log-in")).await?;
    Ok(())
}

#[tokio::test]
async fn tag_02_a_list_about_a_project_holds_only_the_subjects_that_can_reach_it() -> Outcome {
    let g = groups().await?;
    let name_of = |listed: &[crate::issues::IssueTypeView]| -> Vec<String> {
        listed.iter().map(|each| each.name.clone()).collect()
    };
    new_type(&g.world, &g.admin, Some("ACME"), "In ACME").await?;
    new_type(&g.world, &g.admin, Some("OTHER"), "In OTHER").await?;
    new_type(&g.world, &g.admin, Some("SUB"), "In SUB").await?;
    group_in(&g.world, &g.admin, Some("OTHER"), "Elsewhere").await?;
    // The administrator reads everything; a list about SUB holds what reaches SUB: the types of the
    // organization, of ACME above it and of SUB itself.
    let in_sub = name_of(&types(&g.world, &g.admin, Some("SUB")).await?);
    assert!(in_sub.contains(&"In ACME".to_owned()));
    assert!(in_sub.contains(&"In SUB".to_owned()));
    assert!(in_sub.contains(&"Bug".to_owned()));
    assert!(!in_sub.contains(&"In OTHER".to_owned()));
    let in_acme = name_of(&types(&g.world, &g.admin, Some("ACME")).await?);
    assert!(!in_acme.contains(&"In SUB".to_owned()));
    // The same for the groups of labels.
    let groups_in_acme: Vec<String> = all_labels(&g.world, &g.admin, Some("ACME"))
        .await?
        .groups
        .into_iter()
        .map(|each| each.name)
        .collect();
    assert!(groups_in_acme.contains(&"Area".to_owned()));
    assert!(!groups_in_acme.contains(&"Elsewhere".to_owned()));
    // Without a project the list holds everything the person may know.
    let all = name_of(&types(&g.world, &g.admin, None).await?);
    assert!(all.contains(&"In OTHER".to_owned()));
    Ok(())
}

#[tokio::test]
async fn sec_03_a_visible_sub_project_under_a_hidden_parent_knows_what_reaches_it() -> Outcome {
    let g = groups().await?;
    let world = &g.world;
    let create = async |parent: Option<ProjectId>, short: &str| {
        crate::projects::create_project(
            &world.ctx(),
            crate::projects::CreateProject {
                session_token: g.admin.clone(),
                parent,
                short_name: short.to_owned(),
                name: short.to_owned(),
            },
        )
        .await
    };
    let hidden = create(None, "HID").await?.into_value().id;
    let visible = create(Some(hidden), "VIS").await?.into_value().id;
    let inherited = new_type(world, &g.admin, Some("HID"), "Inherited").await?;
    let area = group_in(world, &g.admin, Some("HID"), "Shared area").await?;
    let lead = lead_of(world, "vera@example.test", visible)
        .await?
        .session_token;
    // The lead sees VIS and not HID, and is told what HID created because it reaches VIS.
    let listed = types(world, &lead, Some("VIS")).await?;
    assert!(listed.iter().any(|each| each.id == inherited.id));
    assert!(
        all_labels(world, &lead, Some("VIS"))
            .await?
            .groups
            .iter()
            .any(|each| each.id == area)
    );
    // New issue in VIS offers it, and the lead may switch it off there.
    crate::issues::create_issue(&world.ctx(), new_issue(&lead, "VIS", inherited.id)).await?;
    change(world, &lead, inherited.id, vec![at("VIS", Some(false))]).await?;
    let refused_now =
        crate::issues::create_issue(&world.ctx(), new_issue(&lead, "VIS", inherited.id)).await;
    assert!(refused(&refused_now, ErrorCode::IssueTypeNotOffered));
    // The name of HID itself is not told.
    let created_in = listed
        .iter()
        .find(|each| each.id == inherited.id)
        .and_then(|each| each.created_in.clone());
    assert_eq!(created_in, Some(crate::access::ProjectLabel::Hidden));
    Ok(())
}

#[tokio::test]
async fn sec_03_an_issue_of_a_type_the_writer_does_not_know_is_not_found() -> Outcome {
    let g = groups().await?;
    let question = new_type(&g.world, &g.lead, Some("ACME"), "Question").await?;
    let outside = outsiders(&g).await?;
    let create = crate::issues::create_issue(
        &g.world.ctx(),
        new_issue(&outside.sibling_lead, "OTHER", question.id),
    )
    .await;
    assert!(refused(&create, ErrorCode::NotFound));
    Ok(())
}

#[tokio::test]
async fn sec_06_a_group_in_use_is_unknown_to_a_reader_whose_role_hides_the_labels() -> Outcome {
    use bb_core::access::FieldLevel;
    use bb_core::fields::{Field, FieldKey};

    use crate::roles::{CopyRole, SetFieldLevel, copy_role, set_level};

    let g = groups().await?;
    let world = &g.world;
    // An issue with a label of the group of ACME leaves for OTHER.
    let issue = an_issue(world, g.acme).await?;
    label_on(world, &g.admin, issue, g.login).await?;
    move_issue(
        &world.ctx(),
        MoveIssue {
            session_token: g.admin.clone(),
            issue: issue.to_string(),
            version: 2,
            project: "OTHER".to_owned(),
        },
    )
    .await?;
    // Two readers of OTHER: a Team member, and a copy of that role that does not see the labels.
    let member_role = starting_role(world, StartingRole::TeamMember).await?;
    let blind = copy_role(
        &world.ctx(),
        CopyRole {
            session_token: g.admin.clone(),
            role: member_role,
            name: "Blind to labels".to_owned(),
        },
    )
    .await?
    .into_value();
    set_level(
        &world.ctx(),
        SetFieldLevel {
            session_token: g.admin.clone(),
            role: blind.id,
            project: None,
            issue_type: None,
            field: FieldKey::BuiltIn(Field::Labels),
            level: Some(FieldLevel::Hidden),
        },
    )
    .await?;
    let sighted = member(world, "sia@example.test", StartingRole::TeamMember, g.other).await?;
    let ron = world.person("ron@example.test").await?;
    assign(world, &ron, blind.id, g.other).await?;
    // The one who may see the labels of the issue is told of the group; the other is not.
    let knows = all_labels(world, &sighted.session_token, None).await?;
    assert!(knows.groups.iter().any(|each| each.id == g.area));
    let blind_to = all_labels(world, &ron.session_token, None).await?;
    assert!(blind_to.groups.iter().all(|each| each.id != g.area));
    assert!(blind_to.labels.iter().all(|each| each.id != g.login));
    Ok(())
}

#[tokio::test]
async fn sec_03_a_lead_who_knows_a_project_they_cannot_see_is_told_the_same_by_the_list_and_the_put()
-> Outcome {
    let g = groups().await?;
    let world = &g.world;
    let lia = lead_of(world, "lia@example.test", g.acme).await?;
    crate::roles::assign_role(
        &world.ctx(),
        crate::roles::AssignRole {
            session_token: g.admin.clone(),
            subject: crate::roles::Subject::Person(lia.person.id),
            given: crate::roles::Given::NoAccess,
            project: Some(g.sub),
            applies_to: bb_core::access::AppliesTo::ProjectAndSubProjects,
        },
    )
    .await?;
    let bug = another_type(world).await?;
    // SUB is known to the lead (they decide who has which role there) but not seen: the list about
    // it is answered, and a row for it is refused for the permission, not as a project not there.
    types(world, &lia.session_token, Some("SUB")).await?;
    let put = change(world, &lia.session_token, bug, vec![at("SUB", Some(false))]).await;
    assert!(refused(&put, ErrorCode::Forbidden));
    Ok(())
}
