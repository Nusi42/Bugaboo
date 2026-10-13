//! Tags and labels against the in-memory store (TAG-01, TAG-02, TAG-03): a tag is found by its name
//! whatever its capitals, a *Pick one* group keeps one label of the group on an issue, combining
//! changes every issue once and says so in the history of each, and a person who may not look after
//! labels cannot.

use bb_core::ErrorCode;
use bb_core::event::{DomainEvent, EntityKind, RecordKind};
use bb_core::id::IssueId;
use bb_core::own_fields::ChoiceColor;
use serde_json::json;

use super::scenarios::{a_project, token};
use super::scenarios_issues::an_issue;
use super::scenarios_tags::{a_label, a_label_group, labelled, tagged};
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::tags::{
    AddLabel, AddTag, ChangeLabel, ChangeTag, CombineLabels, CombineTags, IssueSortingQuery,
    LabelChangePreviewQuery, MakeLabel, TagChangePreviewQuery, add_label, add_tag, change_label,
    change_tag, combine_labels, combine_tags, issue_sorting, label_change_preview, make_label,
    tag_change_preview,
};

/// The events about issues that say something about `member`, oldest first.
fn about_issues(world: &World, member: &str) -> Vec<DomainEvent> {
    let issue = EntityKind::from(RecordKind::Issue);
    world
        .store
        .lock()
        .events
        .iter()
        .filter(|event| event.entity.kind == issue)
        .filter(|event| event.changes.iter().any(|change| change.field == member))
        .cloned()
        .collect()
}

async fn names(world: &World, issue: IssueId) -> Result<(Vec<String>, Vec<String>), AppError> {
    let sorting = issue_sorting(
        &world.ctx(),
        IssueSortingQuery {
            session_token: token(world),
            issue: issue.to_string(),
        },
    )
    .await?
    .into_value();
    Ok((
        sorting.tags.iter().map(|tag| tag.name.clone()).collect(),
        sorting
            .labels
            .iter()
            .map(|label| label.name.clone())
            .collect(),
    ))
}

#[tokio::test]
async fn tag_01_a_name_is_one_tag_whatever_its_capitals() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let (first, second) = (
        an_issue(&world, project).await?,
        an_issue(&world, project).await?,
    );
    let a = tagged(&world, first, "Login").await?;
    let b = tagged(&world, second, "  LOGIN ").await?;
    assert_eq!(a, b);
    assert_eq!(names(&world, second).await?.0, vec!["Login".to_owned()]);
    Ok(())
}

#[tokio::test]
async fn tag_01_a_tag_for_other_projects_is_refused_here() -> Outcome {
    let world = World::new().await?;
    let (acme, other) = (
        a_project(&world, "ACME").await?,
        a_project(&world, "ELSE").await?,
    );
    let (here, there) = (
        an_issue(&world, acme).await?,
        an_issue(&world, other).await?,
    );
    let tag = tagged(&world, there, "billing").await?;
    change_tag(
        &world.ctx(),
        ChangeTag {
            session_token: token(&world),
            tag,
            version: 1,
            name: None,
            color: None,
            projects: Some(vec![other]),
        },
    )
    .await?;
    let refused = add_tag(
        &world.ctx(),
        AddTag {
            session_token: token(&world),
            issue: here.to_string(),
            name: "Billing".to_owned(),
        },
    )
    .await
    .err();
    assert_eq!(refused, Some(AppError::Refused(ErrorCode::TagNotHere)));
    Ok(())
}

#[tokio::test]
async fn tag_02_a_pick_one_group_keeps_one_label_per_issue() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?;
    let area = a_label_group(&world, "Area", true).await?;
    let (login, billing) = (
        a_label(&world, "login", Some(area)).await?,
        a_label(&world, "billing", Some(area)).await?,
    );
    let urgent = a_label(&world, "urgent", None).await?;
    labelled(&world, issue, login).await?;
    labelled(&world, issue, urgent).await?;
    labelled(&world, issue, billing).await?;
    assert_eq!(
        names(&world, issue).await?.1,
        vec!["urgent".to_owned(), "billing".to_owned()]
    );
    // The change that put *billing* there says what it replaced.
    let last = about_issues(&world, "labels").pop().ok_or("no event")?;
    assert_eq!(
        last.changes[0].before,
        Some(json!(["login", "urgent"])),
        "{last:?}"
    );
    assert_eq!(last.changes[0].after, Some(json!(["urgent", "billing"])));
    // The same label twice changes nothing and writes nothing.
    let before = world.store.event_count();
    labelled(&world, issue, billing).await?;
    assert_eq!(world.store.event_count(), before);
    Ok(())
}

#[tokio::test]
async fn tag_02_a_group_cannot_become_pick_one_while_an_issue_has_two_of_it() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?;
    let area = a_label_group(&world, "Area", false).await?;
    let (login, billing) = (
        a_label(&world, "login", Some(area)).await?,
        a_label(&world, "billing", Some(area)).await?,
    );
    labelled(&world, issue, login).await?;
    labelled(&world, issue, billing).await?;
    let refused = crate::tags::change_label_group(
        &world.ctx(),
        crate::tags::ChangeLabelGroup {
            session_token: token(&world),
            group: area,
            version: 1,
            name: None,
            pick_one: Some(true),
            retired: None,
        },
    )
    .await
    .err();
    assert_eq!(refused, Some(AppError::Refused(ErrorCode::PickOneConflict)));
    Ok(())
}

#[tokio::test]
async fn tag_03_combining_two_labels_changes_every_issue_once_and_records_it() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let (only_old, only_new, both) = (
        an_issue(&world, project).await?,
        an_issue(&world, project).await?,
        an_issue(&world, project).await?,
    );
    let (old, new) = (
        a_label(&world, "login", None).await?,
        a_label(&world, "sign-in", None).await?,
    );
    labelled(&world, only_old, old).await?;
    labelled(&world, only_new, new).await?;
    labelled(&world, both, old).await?;
    labelled(&world, both, new).await?;
    let already = about_issues(&world, "labels").len();
    let preview = label_change_preview(
        &world.ctx(),
        LabelChangePreviewQuery {
            session_token: token(&world),
            label: old,
            name: None,
            into: Some(new),
        },
    )
    .await?
    .into_value();
    // The issue that has only the new label does not change.
    assert_eq!(preview.summary.will_change, 2);
    combine_labels(
        &world.ctx(),
        CombineLabels {
            session_token: token(&world),
            label: old,
            version: 1,
            into: new,
            into_version: 1,
        },
    )
    .await?;
    for issue in [only_old, only_new, both] {
        assert_eq!(names(&world, issue).await?.1, vec!["sign-in".to_owned()]);
    }
    // One event for each issue that changed, and none for the issue that did not.
    let events = about_issues(&world, "labels");
    assert_eq!(events.len() - already, 2);
    let mut after: Vec<_> = events[already..]
        .iter()
        .map(|event| {
            (
                event.changes[0].before.clone(),
                event.changes[0].after.clone(),
            )
        })
        .collect();
    after.sort_by_key(|(before, _)| before.as_ref().map(ToString::to_string));
    assert_eq!(
        after,
        vec![
            (Some(json!(["login", "sign-in"])), Some(json!(["sign-in"]))),
            (Some(json!(["login"])), Some(json!(["sign-in"]))),
        ]
    );
    Ok(())
}

#[tokio::test]
async fn tag_03_renaming_a_tag_is_in_the_history_of_every_issue_that_has_it() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let (first, second) = (
        an_issue(&world, project).await?,
        an_issue(&world, project).await?,
    );
    let tag = tagged(&world, first, "login").await?;
    tagged(&world, second, "login").await?;
    let already = about_issues(&world, "tags").len();
    let preview = tag_change_preview(
        &world.ctx(),
        TagChangePreviewQuery {
            session_token: token(&world),
            tag,
            name: Some("sign-in".to_owned()),
            into: None,
        },
    )
    .await?
    .into_value();
    assert_eq!(preview.summary.will_change, 2);
    change_tag(
        &world.ctx(),
        ChangeTag {
            session_token: token(&world),
            tag,
            version: 1,
            name: Some("sign-in".to_owned()),
            color: None,
            projects: None,
        },
    )
    .await?;
    for issue in [first, second] {
        assert_eq!(names(&world, issue).await?.0, vec!["sign-in".to_owned()]);
    }
    let events = about_issues(&world, "tags");
    assert_eq!(events.len() - already, 2);
    for event in &events[already..] {
        assert_eq!(event.changes[0].before, Some(json!(["login"])));
        assert_eq!(event.changes[0].after, Some(json!(["sign-in"])));
    }
    // Another tag cannot take the name.
    let third = an_issue(&world, project).await?;
    let other = tagged(&world, third, "slow").await?;
    let refused = change_tag(
        &world.ctx(),
        ChangeTag {
            session_token: token(&world),
            tag: other,
            version: 1,
            name: Some("SIGN-IN".to_owned()),
            color: None,
            projects: None,
        },
    )
    .await
    .err();
    assert_eq!(refused, Some(AppError::Refused(ErrorCode::NameTaken)));
    Ok(())
}

#[tokio::test]
async fn tag_03_combining_tags_needs_the_versions_the_person_looked_at() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?;
    let (a, b) = (
        tagged(&world, issue, "login").await?,
        tagged(&world, issue, "sign-in").await?,
    );
    change_tag(
        &world.ctx(),
        ChangeTag {
            session_token: token(&world),
            tag: a,
            version: 1,
            name: None,
            color: Some(ChoiceColor::Cat7),
            projects: None,
        },
    )
    .await?;
    let stale = combine_tags(
        &world.ctx(),
        CombineTags {
            session_token: token(&world),
            tag: a,
            version: 1,
            into: b,
            into_version: 1,
        },
    )
    .await
    .err();
    assert_eq!(stale, Some(AppError::Refused(ErrorCode::VersionChanged)));
    combine_tags(
        &world.ctx(),
        CombineTags {
            session_token: token(&world),
            tag: a,
            version: 2,
            into: b,
            into_version: 1,
        },
    )
    .await?;
    assert_eq!(names(&world, issue).await?.0, vec!["sign-in".to_owned()]);
    Ok(())
}

#[tokio::test]
async fn tag_02_only_the_people_who_look_after_labels_make_and_change_them() -> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?;
    let refused = make_label(
        &world.ctx(),
        MakeLabel {
            session_token: bob.session_token.clone(),
            name: "login".to_owned(),
            color: None,
            group: None,
        },
    )
    .await
    .err();
    assert_eq!(refused, Some(AppError::Refused(ErrorCode::Forbidden)));
    let label = a_label(&world, "login", None).await?;
    let renamed = change_label(
        &world.ctx(),
        ChangeLabel {
            session_token: bob.session_token,
            label,
            version: 1,
            name: Some("x".to_owned()),
            color: None,
            group: None,
            retired: None,
        },
    )
    .await
    .err();
    assert_eq!(renamed, Some(AppError::Refused(ErrorCode::Forbidden)));
    Ok(())
}

#[tokio::test]
async fn tag_02_a_retired_label_is_not_put_on_an_issue() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?;
    let label = a_label(&world, "old", None).await?;
    change_label(
        &world.ctx(),
        ChangeLabel {
            session_token: token(&world),
            label,
            version: 1,
            name: None,
            color: None,
            group: None,
            retired: Some(true),
        },
    )
    .await?;
    let refused = add_label(
        &world.ctx(),
        AddLabel {
            session_token: token(&world),
            issue: issue.to_string(),
            label,
        },
    )
    .await
    .err();
    assert_eq!(refused, Some(AppError::Refused(ErrorCode::InvalidRequest)));
    Ok(())
}
