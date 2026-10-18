//! The scenarios of the tag and label use cases (ADR-0007, M1-F18): the preparation, the call, and
//! how many events the call added. `events.rs` names each use case once and runs them.

use bb_core::id::{IssueId, LabelGroupId, LabelId, TagId};
use bb_core::names::name_key;
use bb_core::own_fields::{ChoiceColor, Edit};

use super::scenarios::{Added, a_project, measured, text, token};
use super::scenarios_issues::an_issue;
use super::world::World;
use crate::tags::{
    AddLabel, AddTag, ChangeLabel, ChangeLabelGroup, ChangeTag, CombineLabels, CombineTags,
    CreateLabel, CreateLabelGroup, RemoveLabel, RemoveTag, add_label, add_tag, change_label,
    change_label_group, change_tag, combine_labels, combine_tags, create_label, create_label_group,
    remove_label, remove_tag,
};

/// A group of labels, made by the administrator.
pub(crate) async fn a_label_group(
    world: &World,
    name: &str,
    pick_one: bool,
) -> Result<LabelGroupId, String> {
    Ok(create_label_group(
        &world.ctx(),
        CreateLabelGroup {
            project: None,
            session_token: token(world),
            name: name.to_owned(),
            pick_one,
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id)
}

/// A label, made by the administrator.
pub(crate) async fn a_label(
    world: &World,
    name: &str,
    group: Option<LabelGroupId>,
) -> Result<LabelId, String> {
    Ok(create_label(
        &world.ctx(),
        CreateLabel {
            session_token: token(world),
            name: name.to_owned(),
            color: None,
            group,
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id)
}

/// Puts a tag on an issue, as the administrator, and says which tag it is.
pub(crate) async fn tagged(world: &World, issue: IssueId, name: &str) -> Result<TagId, String> {
    let labels_and_tags = add_tag(
        &world.ctx(),
        AddTag {
            session_token: token(world),
            issue: issue.to_string(),
            name: name.to_owned(),
        },
    )
    .await
    .map_err(text)?
    .into_value();
    // The next one comes a moment later, so the order they came in is the order they are read in.
    world.clock.advance(1);
    labels_and_tags
        .tags
        .iter()
        .find(|tag| name_key(&tag.name) == name_key(name))
        .map(|tag| tag.id)
        .ok_or_else(|| "the tag is not on the issue".to_owned())
}

/// Puts a label on an issue, as the administrator.
pub(crate) async fn labelled(world: &World, issue: IssueId, label: LabelId) -> Result<(), String> {
    add_label(
        &world.ctx(),
        AddLabel {
            session_token: token(world),
            issue: issue.to_string(),
            label,
        },
    )
    .await
    .map_err(text)?;
    world.clock.advance(1);
    Ok(())
}

pub(crate) async fn add_tag_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    measured(
        &world.store,
        add_tag(
            &world.ctx(),
            AddTag {
                session_token: token(world),
                issue: issue.to_string(),
                name: "login".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn remove_tag_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    let tag = tagged(world, issue, "login").await?;
    measured(
        &world.store,
        remove_tag(
            &world.ctx(),
            RemoveTag {
                session_token: token(world),
                issue: issue.to_string(),
                tag,
            },
        ),
    )
    .await
}

pub(crate) async fn change_tag_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    let tag = tagged(world, issue, "login").await?;
    // The one event of the tag, and one for the issue that has it.
    let before = world.store.event_count();
    change_tag(
        &world.ctx(),
        ChangeTag {
            session_token: token(world),
            tag,
            version: 1,
            name: None,
            color: Some(ChoiceColor::Cat5),
            projects: None,
        },
    )
    .await
    .map_err(text)?;
    Ok(world.store.event_count() - before)
}

pub(crate) async fn combine_tags_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    let other = an_issue(world, project).await?;
    let (gone, kept) = (
        tagged(world, issue, "login").await?,
        tagged(world, other, "sign-in").await?,
    );
    // No issue has the tag that goes away: the combine writes the one event of the tag.
    remove_tag(
        &world.ctx(),
        RemoveTag {
            session_token: token(world),
            issue: issue.to_string(),
            tag: gone,
        },
    )
    .await
    .map_err(text)?;
    measured(
        &world.store,
        combine_tags(
            &world.ctx(),
            CombineTags {
                session_token: token(world),
                tag: gone,
                version: 1,
                into: kept,
                into_version: 1,
            },
        ),
    )
    .await
}

pub(crate) async fn create_label_group_scenario(world: &World) -> Added {
    measured(
        &world.store,
        create_label_group(
            &world.ctx(),
            CreateLabelGroup {
                project: None,
                session_token: token(world),
                name: "Area".to_owned(),
                pick_one: true,
            },
        ),
    )
    .await
}

pub(crate) async fn change_label_group_scenario(world: &World) -> Added {
    let group = a_label_group(world, "Area", false).await?;
    measured(
        &world.store,
        change_label_group(
            &world.ctx(),
            ChangeLabelGroup {
                session_token: token(world),
                group,
                version: 1,
                name: Some("Part".to_owned()),
                pick_one: None,
                retired: None,
            },
        ),
    )
    .await
}

pub(crate) async fn create_label_scenario(world: &World) -> Added {
    measured(
        &world.store,
        create_label(
            &world.ctx(),
            CreateLabel {
                session_token: token(world),
                name: "login".to_owned(),
                color: None,
                group: None,
            },
        ),
    )
    .await
}

pub(crate) async fn change_label_scenario(world: &World) -> Added {
    let label = a_label(world, "login", None).await?;
    measured(
        &world.store,
        change_label(
            &world.ctx(),
            ChangeLabel {
                session_token: token(world),
                label,
                version: 1,
                name: Some("sign-in".to_owned()),
                color: None,
                group: Edit::Keep,
                retired: None,
            },
        ),
    )
    .await
}

pub(crate) async fn combine_labels_scenario(world: &World) -> Added {
    let (gone, kept) = (
        a_label(world, "login", None).await?,
        a_label(world, "sign-in", None).await?,
    );
    measured(
        &world.store,
        combine_labels(
            &world.ctx(),
            CombineLabels {
                session_token: token(world),
                label: gone,
                version: 1,
                into: kept,
                into_version: 1,
            },
        ),
    )
    .await
}

pub(crate) async fn add_label_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    let label = a_label(world, "login", None).await?;
    measured(
        &world.store,
        add_label(
            &world.ctx(),
            AddLabel {
                session_token: token(world),
                issue: issue.to_string(),
                label,
            },
        ),
    )
    .await
}

pub(crate) async fn remove_label_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    let label = a_label(world, "login", None).await?;
    labelled(world, issue, label).await?;
    measured(
        &world.store,
        remove_label(
            &world.ctx(),
            RemoveLabel {
                session_token: token(world),
                issue: issue.to_string(),
                label,
            },
        ),
    )
    .await
}
