//! The two release fields of an issue (ISS-08, ADR-0049, decision 3, AI-21) on the in-memory
//! store: the limit of *Found in release*, the one rule of a choice on every way in, the events of
//! the issue, an issue that moved out of the tree, and the tools of AI apps.

use bb_core::ErrorCode;
use bb_core::event::{EntityKind, RecordKind};
use bb_core::given::SetTo;
use bb_core::project_release::FOUND_IN_RELEASES_MAX;
use bb_policy::StartingRole;
use serde_json::{Value, json};

use super::ai_tools::call;
use super::releases::{
    a_release, a_sub_project, changing, last_release_event, previewing, refused, seen, set_releases,
};
use super::scenarios::{a_project, text, token};
use super::scenarios_issues::an_issue;
use super::scenarios_searches::with_role;
use super::world::{Outcome, World};
use crate::ai::tools::call_tool;
use crate::issues::{
    ChangeIssue, CreateIssue, IssueView, MoveIssue, change_issue, create_issue, move_issue,
};
use crate::many_changes::{
    ManyChange, ManyChangeAction, ManyChangePreviewQuery, preview_many_changes,
};
use crate::releases::{ChangeRelease, change_release};

#[tokio::test]
async fn iss_08_found_in_takes_up_to_20_releases_and_planned_for_one() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "APP").await?;
    let mut made = Vec::new();
    for at in 0..=FOUND_IN_RELEASES_MAX {
        made.push(a_release(&world, "APP", &format!("1.{at}")).await?.id);
    }
    let issue = an_issue(&world, project).await?;
    refused(
        set_releases(&world, issue, Some(made.clone()), None).await,
        ErrorCode::ReleasesTooMany,
    );
    let most = made[..FOUND_IN_RELEASES_MAX].to_vec();
    let view = set_releases(&world, issue, Some(most.clone()), None).await?;
    assert_eq!(
        view.found_in_release
            .unwrap_or_default()
            .iter()
            .map(|each| each.id)
            .collect::<Vec<_>>(),
        most
    );
    // A release given twice counts once.
    let view = set_releases(&world, issue, Some(vec![made[1], made[0], made[1]]), None).await?;
    assert_eq!(
        view.found_in_release
            .unwrap_or_default()
            .iter()
            .map(|each| each.id)
            .collect::<Vec<_>>(),
        [made[1], made[0]]
    );
    let view = set_releases(&world, issue, None, Some(SetTo::Value(made[2]))).await?;
    assert_eq!(view.planned_for_release.map(|each| each.id), Some(made[2]));
    let view = set_releases(&world, issue, None, Some(SetTo::Nothing)).await?;
    assert!(view.planned_for_release.is_none());
    Ok(())
}

#[tokio::test]
async fn iss_08_a_release_not_offered_where_the_issue_is_is_refused_on_every_way_in() -> Outcome {
    let world = World::new().await?;
    let here = a_project(&world, "APP").await?;
    a_project(&world, "OPS").await?;
    let elsewhere = a_release(&world, "OPS", "9.0").await?;
    // The form and the API: making an issue.
    let kind = super::scenarios_issues::a_type(&world).await?;
    refused(
        create_issue(
            &world.ctx(),
            CreateIssue {
                planned_for_release: Some(elsewhere.id),
                ..CreateIssue::of(&token(&world), "APP", kind, "Crash")
            },
        )
        .await,
        ErrorCode::ReleaseNotOffered,
    );
    refused(
        create_issue(
            &world.ctx(),
            CreateIssue {
                found_in_release: Some(vec![elsewhere.id]),
                ..CreateIssue::of(&token(&world), "APP", kind, "Crash")
            },
        )
        .await,
        ErrorCode::ReleaseNotOffered,
    );
    // Changing an issue.
    let issue = an_issue(&world, here).await?;
    refused(
        set_releases(&world, issue, Some(vec![elsewhere.id]), None).await,
        ErrorCode::ReleaseNotOffered,
    );
    // A change of many issues leaves the issue alone.
    let preview = preview_many_changes(
        &world.ctx(),
        ManyChangePreviewQuery {
            session_token: token(&world),
            issues: vec![issue.to_string()],
            action: ManyChangeAction::Change {
                change: ManyChange {
                    planned_for_release: Some(SetTo::Value(elsewhere.id)),
                    ..ManyChange::default()
                },
            },
        },
    )
    .await
    .map_err(text)?
    .into_value();
    assert!(preview.rows[0].skipped.is_some(), "{:?}", preview.rows[0]);
    // The tool of AI apps and the assistant.
    let version = seen(&world, issue).await?.version;
    let answer = call_tool(
        &world.ctx(),
        previewing(
            &token(&world),
            "update_issue",
            json!({"issue": issue.to_string(), "version": version, "planned_for_release": elsewhere.id.to_string()}),
        ),
    )
    .await;
    refused(answer, ErrorCode::ReleaseNotOffered);
    Ok(())
}

#[tokio::test]
async fn iss_08_every_release_change_is_an_event_with_old_and_new_values() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "APP").await?;
    let made = a_release(&world, "APP", "2.4").await?;
    let event = last_release_event(&world).ok_or("no event of the new release")?;
    assert!(event.changes.iter().any(|change| change.field == "name"
        && change.before.is_none()
        && change.after == Some(json!("2.4"))));
    let renamed = change_release(
        &world.ctx(),
        ChangeRelease {
            name: Some("2.4.0".to_owned()),
            release_date: Some(SetTo::Value("2026-12-01".parse().map_err(text)?)),
            ..changing(&token(&world), "APP", &made)
        },
    )
    .await
    .map_err(text)?
    .into_value();
    let event = last_release_event(&world).ok_or("no event of the change")?;
    let change = |field: &str| {
        event
            .changes
            .iter()
            .find(|change| change.field == field)
            .cloned()
    };
    let name = change("name").ok_or("no name")?;
    assert_eq!(
        (name.before, name.after),
        (Some(json!("2.4")), Some(json!("2.4.0")))
    );
    let date = change("release-date").ok_or("no date")?;
    assert_eq!(
        (date.before, date.after),
        (Some(json!(null)), Some(json!("2026-12-01")))
    );
    assert!(change("version").is_some());
    // A change that changes nothing writes nothing.
    let before = world.store.event_count();
    change_release(
        &world.ctx(),
        ChangeRelease {
            name: Some("2.4.0".to_owned()),
            ..changing(&token(&world), "APP", &renamed)
        },
    )
    .await
    .map_err(text)?;
    assert_eq!(world.store.event_count(), before);
    // On the issue: both fields with their old and new values, the names as they were.
    let issue = an_issue(&world, project).await?;
    set_releases(
        &world,
        issue,
        Some(vec![made.id]),
        Some(SetTo::Value(made.id)),
    )
    .await?;
    let issue_event = world
        .store
        .lock()
        .events
        .iter()
        .rev()
        .find(|event| event.entity.kind == EntityKind::from(RecordKind::Issue))
        .cloned()
        .ok_or("no event of the issue")?;
    let field = |name: &str| {
        issue_event
            .changes
            .iter()
            .find(|change| change.field == name)
            .cloned()
    };
    let found = field("found-in-release").ok_or("no found in release")?;
    assert_eq!(found.before, Some(json!([])));
    let after: &Value = found.after.as_ref().ok_or("no value after")?;
    assert_eq!(after[0]["name"], json!("2.4.0"));
    let planned = field("planned-for-release").ok_or("no planned for release")?;
    assert_eq!(planned.before, None);
    assert_eq!(
        planned.after.as_ref().map(|value| value["id"].clone()),
        Some(json!(made.id.to_string()))
    );
    Ok(())
}

#[tokio::test]
async fn iss_08_an_issue_moved_out_of_the_tree_keeps_its_release_marked_not_offered() -> Outcome {
    let world = World::new().await?;
    let top = a_project(&world, "APP").await?;
    let web = a_sub_project(&world, top, "WEB").await?;
    a_project(&world, "OPS").await?;
    let release = a_release(&world, "APP", "2.4").await?;
    let issue = an_issue(&world, web).await?;
    let view = set_releases(
        &world,
        issue,
        Some(vec![release.id]),
        Some(SetTo::Value(release.id)),
    )
    .await?;
    assert!(view.found_in_release.clone().unwrap_or_default()[0].offered);
    move_issue(
        &world.ctx(),
        MoveIssue {
            session_token: token(&world),
            issue: issue.to_string(),
            version: view.version,
            project: "OPS".to_owned(),
        },
    )
    .await
    .map_err(text)?;
    let moved = seen(&world, issue).await?;
    assert_eq!(moved.found_in_release.clone().unwrap_or_default().len(), 1);
    assert!(!moved.found_in_release.clone().unwrap_or_default()[0].offered);
    let planned = moved.planned_for_release.ok_or("the release is gone")?;
    assert!(!planned.offered);
    // Another field of it still changes, and the release stays.
    let after = change_issue::<_, IssueView>(
        &world.ctx(),
        ChangeIssue {
            title: Some("Moved".to_owned()),
            ..ChangeIssue::of(&token(&world), issue.to_string(), moved.version)
        },
    )
    .await
    .map_err(text)?
    .into_value();
    assert_eq!(
        after.planned_for_release.map(|each| each.id),
        Some(release.id)
    );
    Ok(())
}

#[tokio::test]
async fn ai_21_list_releases_reads_only_releases_of_projects_the_person_sees() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "APP").await?;
    a_project(&world, "HID").await?;
    a_release(&world, "APP", "2.4").await?;
    a_release(&world, "HID", "secret-7").await?;
    let member = world.person("tea@example.test").await.map_err(text)?;
    with_role(&world, &member, StartingRole::TeamMember, Some(project)).await?;
    let answer = call_tool(
        &world.ctx(),
        call(
            &member.session_token,
            "list_releases",
            json!({"project": "APP"}),
        ),
    )
    .await
    .map_err(text)?
    .into_value();
    assert!(answer.text.contains("2.4"), "{}", answer.text);
    assert!(!answer.text.contains("secret-7"), "{}", answer.text);
    let hidden = call_tool(
        &world.ctx(),
        call(
            &member.session_token,
            "list_releases",
            json!({"project": "HID"}),
        ),
    )
    .await;
    refused(hidden, ErrorCode::NotFound);
    // The tool only reads: no tool of the registry makes, marks or deletes a release.
    let tools = crate::ai::tools::registry::<super::fake::FakeStore>();
    let listing = tools
        .iter()
        .find(|spec| spec.name() == "list_releases")
        .ok_or("no tool list_releases")?;
    assert_eq!(listing.effect(), bb_core::ai::ToolEffect::Reads);
    assert!(
        tools.iter().all(|spec| !spec.name().contains("release")
            || spec.effect() == bb_core::ai::ToolEffect::Reads)
    );
    Ok(())
}

#[tokio::test]
async fn ai_21_change_issue_sets_planned_for_release_with_its_preview() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "APP").await?;
    let release = a_release(&world, "APP", "2.4").await?;
    let issue = an_issue(&world, project).await?;
    let version = seen(&world, issue).await?.version;
    let input = json!({
        "issue": issue.to_string(),
        "version": version,
        "planned_for_release": release.id.to_string(),
    });
    // The assistant proposes: the preview names the field and the release, and nothing is saved.
    let preview = call_tool(
        &world.ctx(),
        previewing(&token(&world), "update_issue", input),
    )
    .await
    .map_err(text)?
    .into_value();
    let change = preview
        .changes
        .iter()
        .find(|change| change.field == "planned-for-release")
        .ok_or_else(|| format!("no change of the release: {:?}", preview.changes))?;
    assert!(
        change
            .after
            .as_ref()
            .is_some_and(|after| after.to_string().contains("2.4")),
        "{change:?}"
    );
    assert!(seen(&world, issue).await?.planned_for_release.is_none());
    Ok(())
}

/// A person whose role sees and changes the issues of `project` but neither release field.
async fn reader_without_releases(
    world: &World,
    project: bb_core::id::ProjectId,
) -> Result<crate::auth::Started, String> {
    use bb_core::access::{AppliesTo, FieldLevel};
    use bb_core::fields::Field;
    use bb_policy::Permission;

    use crate::roles::{AssignRole, CreateRole, Given, Subject, assign_role, create_role};

    // A role that sees and changes issues but neither release field.
    let blind = create_role(
        &world.ctx(),
        CreateRole {
            session_token: token(world),
            name: "No releases".to_owned(),
            permissions: vec![
                Permission::SeeProjects,
                Permission::SeeIssues,
                Permission::ChangeIssues,
            ],
            two_step: false,
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id;
    super::notifications::levels(
        world,
        blind,
        &[Field::FoundInRelease, Field::PlannedForRelease],
        FieldLevel::Hidden,
    )
    .await?;
    let dora = world.person("dora@example.test").await.map_err(text)?;
    assign_role(
        &world.ctx(),
        AssignRole {
            session_token: token(world),
            subject: Subject::Person(dora.person.id),
            given: Given::Role(blind),
            project: Some(project),
            applies_to: AppliesTo::ProjectAndSubProjects,
        },
    )
    .await
    .map_err(text)?;
    Ok(dora)
}

#[tokio::test]
async fn iss_08_the_two_fields_follow_field_permissions_everywhere() -> Outcome {
    use bb_core::history::HistoryTab;

    use crate::history::{HistoryQuery, history};
    use crate::search::{ReadableFieldsQuery, SearchScope, readable_fields};

    let world = World::new().await?;
    let project = a_project(&world, "APP").await?;
    let release = a_release(&world, "APP", "Moonbeam").await?;
    let issue = an_issue(&world, project).await?;
    set_releases(
        &world,
        issue,
        Some(vec![release.id]),
        Some(SetTo::Value(release.id)),
    )
    .await?;
    let dora = reader_without_releases(&world, project).await?;
    let session = dora.session_token.as_str();
    // The issue: neither member in the answer.
    let read = crate::issues::issue(
        &world.ctx(),
        crate::issues::IssueQuery {
            session_token: session.to_owned(),
            issue: issue.to_string(),
        },
    )
    .await
    .map_err(text)?
    .masked_value()
    .map_err(text)?;
    assert!(read["issue"].get("found_in_release").is_none(), "{read}");
    assert!(read["issue"].get("planned_for_release").is_none(), "{read}");
    assert!(!read.to_string().contains("Moonbeam"), "{read}");
    // Search: neither word is a field Dora may use.
    let fields = readable_fields(
        &world.ctx(),
        ReadableFieldsQuery {
            session_token: session.to_owned(),
            scope: SearchScope::Within(project),
        },
    )
    .await
    .map_err(text)?
    .into_value();
    assert!(
        fields
            .0
            .fields
            .iter()
            .all(|field| !field.word.contains("release")),
        "{:?}",
        fields.0.fields
    );
    // History: the change of the releases shows nothing to her.
    let rows = history(
        &world.ctx(),
        HistoryQuery {
            session_token: session.to_owned(),
            issue: issue.to_string(),
            tab: HistoryTab::Changes,
            cursor: None,
            limit: 50,
        },
    )
    .await
    .map_err(text)?
    .masked_value()
    .map_err(text)?;
    assert!(!rows.to_string().contains("Moonbeam"), "{rows}");
    let own = history(
        &world.ctx(),
        HistoryQuery {
            session_token: token(&world),
            issue: issue.to_string(),
            tab: HistoryTab::Changes,
            cursor: None,
            limit: 50,
        },
    )
    .await
    .map_err(text)?
    .masked_value()
    .map_err(text)?;
    assert!(own.to_string().contains("Moonbeam"), "{own}");
    // A change of either field is refused, whatever the value.
    let version = seen(&world, issue).await?.version;
    let refused_change = change_issue::<_, IssueView>(
        &world.ctx(),
        ChangeIssue {
            planned_for_release: Some(SetTo::Nothing),
            ..ChangeIssue::of(session, issue.to_string(), version)
        },
    )
    .await;
    refused(refused_change, ErrorCode::Forbidden);
    // MCP: the tool that reads the issue tells nothing of the releases either.
    let answer = call_tool(
        &world.ctx(),
        call(session, "get_issue", json!({"issue": issue.to_string()})),
    )
    .await
    .map_err(text)?
    .into_value();
    assert!(!answer.text.contains("Moonbeam"), "{}", answer.text);
    Ok(())
}
