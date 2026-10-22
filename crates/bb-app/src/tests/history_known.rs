//! The history and the notifications name an issue type or a label only where the reader may know
//! it (LM1 of the M4 exit review, SEC-03, SEC-06): one rule, `offers/known.rs`. An issue that was
//! of a type made in ACME and moves to OTHER does not tell OTHER the name of that type or of its
//! labels.

use std::collections::BTreeMap;
use std::future::Future;

use crate::access::StartingRole;
use bb_core::history::HistoryTab;
use bb_core::id::{IssueId, IssueTypeId};
use bb_core::message::{self, Message};

use super::offers::{Groups, groups, label_on, lead_of};
use super::scenarios_issues::{a_type, an_issue};
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::history::{HistoryQuery, history};
use crate::issues::{ChangeIssue, IssueQuery, MoveIssue, change_issue, issue, move_issue};
use crate::ports::ChangeStore as _;
use crate::tags::{RemoveLabel, remove_label};

async fn version_of(world: &World, who: &str, issue_id: IssueId) -> Result<i64, AppError> {
    Ok(issue(
        &world.ctx(),
        IssueQuery {
            session_token: who.to_owned(),
            issue: issue_id.to_string(),
        },
    )
    .await?
    .into_value()
    .issue
    .version)
}

async fn type_to(
    world: &World,
    who: &str,
    issue_id: IssueId,
    issue_type: IssueTypeId,
) -> Result<(), AppError> {
    change_issue(
        &world.ctx(),
        ChangeIssue {
            session_token: who.to_owned(),
            issue: issue_id.to_string(),
            version: version_of(world, who, issue_id).await?,
            issue_type: Some(issue_type),
            title: None,
            status: None,
            priority: None,
            assigned_to: None,
            fields: BTreeMap::new(),
        },
    )
    .await?;
    Ok(())
}

/// What the history of the issue says to the reader, as one text (the debug form of its rows).
async fn read(world: &World, who: &str, issue_id: IssueId) -> Result<String, AppError> {
    let page = history(
        &world.ctx(),
        HistoryQuery {
            session_token: who.to_owned(),
            issue: issue_id.to_string(),
            tab: HistoryTab::Changes,
            cursor: None,
            limit: 50,
        },
    )
    .await?
    .into_value();
    Ok(format!("{:?}", page.items))
}

#[tokio::test]
async fn sec_03_the_history_names_a_type_and_a_label_only_where_the_reader_may_know_them() -> Outcome
{
    let Groups {
        world,
        admin,
        lead,
        acme,
        other,
        login,
        ..
    } = groups().await?;
    let question = super::offers_secrecy::new_type(&world, &lead, Some("ACME"), "Question").await?;
    let task = a_type(&world).await?;
    let issue_id = an_issue(&world, acme).await?;
    // The issue was a Question with the label login, then became a Task, and moved to OTHER.
    type_to(&world, &admin, issue_id, question.id).await?;
    label_on(&world, &admin, issue_id, login).await?;
    type_to(&world, &admin, issue_id, task).await?;
    remove_label(
        &world.ctx(),
        RemoveLabel {
            session_token: admin.clone(),
            issue: issue_id.to_string(),
            label: login,
        },
    )
    .await?;
    move_issue(
        &world.ctx(),
        MoveIssue {
            session_token: admin.clone(),
            issue: issue_id.to_string(),
            version: version_of(&world, &admin, issue_id).await?,
            project: "OTHER".to_owned(),
        },
    )
    .await?;
    let outsider = lead_of(&world, "sam@example.test", other)
        .await?
        .session_token;

    // The administrator knows everything and reads both names.
    let inside = read(&world, &admin, issue_id).await?;
    assert!(inside.contains("Question"), "{inside}");
    assert!(inside.contains("names: [\"login\"]"), "{inside}");
    // Who works in OTHER reads neither, and a neutral word where the type was.
    let outside = read(&world, &outsider, issue_id).await?;
    assert!(!outside.contains("Question"), "{outside}");
    assert!(!outside.contains("names: [\"login\"]"), "{outside}");
    let neutral = Message::from(message::HISTORY_TYPE_UNKNOWN).render();
    assert!(outside.contains(&neutral), "{outside}");
    Ok(())
}

/// Makes the label `name` in the group *Area* of ACME.
async fn area_label(g: &Groups, name: &str) -> Result<(bb_core::id::LabelId, i64), AppError> {
    let made = crate::tags::create_label(
        &g.world.ctx(),
        crate::tags::CreateLabel {
            session_token: g.lead.clone(),
            name: name.to_owned(),
            color: None,
            group: Some(g.area),
        },
    )
    .await?
    .into_value();
    Ok((made.id, made.version))
}

/// What the lead of OTHER, who follows the issue, was told in the three ways the issue is told:
/// its history, its notifications and its e-mail, as one text.
async fn all_that_was_told(
    g: &Groups,
    who: &crate::auth::Started,
    issue_id: IssueId,
) -> Result<String, AppError> {
    g.world
        .clock
        .advance(bb_core::notification::MAIL_BATCH_SECONDS + 1);
    while g.world.ctx().run_due_jobs().await? > 0 {}
    let listed = crate::notifications::notifications(
        &g.world.ctx(),
        crate::notifications::NotificationsQuery {
            session_token: who.session_token.clone(),
            filter: bb_core::notification::NotificationFilter::All,
            cursor: None,
            limit: 50,
        },
    )
    .await?
    .into_value()
    .items;
    let mails: Vec<String> = g
        .world
        .mailer
        .messages()
        .into_iter()
        .filter(|mail| mail.to == "sam@example.test")
        .map(|mail| format!("{} {}", mail.subject, mail.text))
        .collect();
    Ok(format!(
        "{} {:?} {:?}",
        read(&g.world, &who.session_token, issue_id).await?,
        listed,
        mails
    ))
}

/// Switches on e-mail, and the news of fields in the app and by e-mail, for everybody.
async fn news_of_fields_everywhere(g: &Groups) -> Result<(), AppError> {
    super::scenarios_notifications::switch_mail_on(&g.world)
        .await
        .map_err(AppError::Internal)?;
    for via in [
        bb_core::notification::Via::App,
        bb_core::notification::Via::Mail,
    ] {
        crate::settings::change_setting(
            &g.world.ctx(),
            crate::settings::ChangeSetting {
                session_token: g.admin.clone(),
                scope: crate::settings::SettingsScope::Organization,
                key: bb_core::notification::NotificationKind::FollowedFields
                    .setting(via)
                    .definition()
                    .key()
                    .to_owned(),
                value: bb_core::settings::SettingValue::Bool(true),
            },
        )
        .await?;
    }
    Ok(())
}

/// An issue that carries the label while it moves to OTHER, where `sam` follows it; then
/// `change` happens to the label, the label is taken off and the group is unused again.
async fn label_change_after_a_move<F, Fut>(
    g: &Groups,
    label: bb_core::id::LabelId,
    change: F,
) -> Result<(String, IssueId), AppError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<bb_core::id::LabelId, AppError>>,
{
    news_of_fields_everywhere(g).await?;
    let sam = lead_of(&g.world, "sam@example.test", g.other).await?;
    let issue_id = an_issue(&g.world, g.acme)
        .await
        .map_err(AppError::Internal)?;
    label_on(&g.world, &g.admin, issue_id, label).await?;
    move_issue(
        &g.world.ctx(),
        MoveIssue {
            session_token: g.admin.clone(),
            issue: issue_id.to_string(),
            version: version_of(&g.world, &g.admin, issue_id).await?,
            project: "OTHER".to_owned(),
        },
    )
    .await?;
    crate::follow::follow_issue(
        &g.world.ctx(),
        crate::follow::FollowIssue {
            session_token: sam.session_token.clone(),
            issue: issue_id.to_string(),
        },
    )
    .await?;
    g.world.clock.advance(1);
    let on_it = change().await?;
    remove_label(
        &g.world.ctx(),
        RemoveLabel {
            session_token: g.admin.clone(),
            issue: issue_id.to_string(),
            label: on_it,
        },
    )
    .await?;
    let told = all_that_was_told(g, &sam, issue_id).await?;
    Ok((told, issue_id))
}

#[tokio::test]
async fn sec_lm1_a_renamed_label_of_a_group_the_reader_does_not_know_shows_no_name() -> Outcome {
    let g = groups().await?;
    let (mango, version) = area_label(&g, "mango").await?;
    let (told, issue_id) = label_change_after_a_move(&g, mango, || async {
        crate::tags::change_label(
            &g.world.ctx(),
            crate::tags::ChangeLabel {
                session_token: g.admin.clone(),
                label: mango,
                version,
                name: Some("papaya".to_owned()),
                color: None,
                group: bb_core::own_fields::Edit::Keep,
                retired: None,
            },
        )
        .await?;
        Ok(mango)
    })
    .await?;
    assert!(!told.contains("mango"), "{told}");
    assert!(!told.contains("papaya"), "{told}");
    // The administrator knows the group, and reads the names the issue went through.
    let known = read(&g.world, &g.admin, issue_id).await?;
    assert!(known.contains("mango"), "{known}");
    Ok(())
}

#[tokio::test]
async fn sec_lm1_a_combined_label_of_a_group_the_reader_does_not_know_shows_no_name() -> Outcome {
    let g = groups().await?;
    let (mango, version) = area_label(&g, "mango").await?;
    let (papaya, into_version) = area_label(&g, "papaya").await?;
    let (told, issue_id) = label_change_after_a_move(&g, mango, || async {
        crate::tags::combine_labels(
            &g.world.ctx(),
            crate::tags::CombineLabels {
                session_token: g.admin.clone(),
                label: mango,
                version,
                into: papaya,
                into_version,
            },
        )
        .await?;
        Ok(papaya)
    })
    .await?;
    assert!(!told.contains("mango"), "{told}");
    assert!(!told.contains("papaya"), "{told}");
    // The administrator knows the group, and reads the names the issue went through.
    let known = read(&g.world, &g.admin, issue_id).await?;
    assert!(known.contains("mango"), "{known}");
    Ok(())
}

#[tokio::test]
async fn sec_lm1_an_old_event_that_holds_a_name_only_is_hidden_when_no_label_has_it_any_more()
-> Outcome {
    let g = groups().await?;
    let sam = lead_of(&g.world, "sam@example.test", g.other).await?;
    let issue_id = an_issue(&g.world, g.acme)
        .await
        .map_err(AppError::Internal)?;
    // An event as M4 wrote it before the groups were kept: names only, of a label that is gone.
    let event = g.world.ctx().event(
        bb_core::event::Actor::System,
        crate::auth::entity(bb_core::event::RecordKind::Issue, issue_id),
        vec![bb_core::event::FieldChange {
            field: "labels".to_owned(),
            before: Some(serde_json::json!(["ghost"])),
            after: Some(serde_json::json!([])),
        }],
    )?;
    g.world
        .ctx()
        .store()
        .record_change(&event, |_| Box::pin(async { Ok(()) }))
        .await
        .map_err(|error| AppError::Internal(format!("{error:?}")))?;
    move_issue(
        &g.world.ctx(),
        MoveIssue {
            session_token: g.admin.clone(),
            issue: issue_id.to_string(),
            version: version_of(&g.world, &g.admin, issue_id).await?,
            project: "OTHER".to_owned(),
        },
    )
    .await?;
    let told = read(&g.world, &sam.session_token, issue_id).await?;
    assert!(!told.contains("ghost"), "{told}");
    // The administrator may know every group, and reads it.
    assert!(read(&g.world, &g.admin, issue_id).await?.contains("ghost"));
    Ok(())
}

/// What the e-mails to `address` said, subject and text, as one text.
fn mails_to(world: &World, address: &str) -> String {
    world
        .mailer
        .messages()
        .into_iter()
        .filter(|mail| mail.to == address)
        .flat_map(|mail| {
            [
                mail.subject,
                " ".to_owned(),
                mail.text,
                "
"
                .to_owned(),
            ]
        })
        .collect()
}

/// Two people follow an issue that went to OTHER: `ann` also leads THIRD, where an issue
/// carries a label of the group *Area* of ACME, so for her the group is in use; `sam` leads OTHER only. Then a label of that group is renamed. Both e-mails go
/// in one round of the jobs, so one context could serve them both. Returns the e-mails of each,
/// as texts, in the order (ann, sam).
async fn two_recipients_in_one_round(
    ann_follows_first: bool,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let g = groups().await?;
    let (mango, version) = area_label(&g, "mango").await?;
    news_of_fields_everywhere(&g).await?;
    let sam = lead_of(&g.world, "sam@example.test", g.other).await?;
    let ann = lead_of(&g.world, "ann@example.test", g.other).await?;
    let lead = super::workflows_rules::starting_role(&g.world, StartingRole::ProjectLead)
        .await
        .map_err(AppError::Internal)?;
    let third = super::scenarios::a_project(&g.world, "THIRD")
        .await
        .map_err(AppError::Internal)?;
    super::workflows_rules::assign(&g.world, &ann, lead, third).await?;
    let issue_id = an_issue(&g.world, g.acme)
        .await
        .map_err(AppError::Internal)?;
    label_on(&g.world, &g.admin, issue_id, mango).await?;
    move_issue(
        &g.world.ctx(),
        MoveIssue {
            session_token: g.admin.clone(),
            issue: issue_id.to_string(),
            version: version_of(&g.world, &g.admin, issue_id).await?,
            project: "OTHER".to_owned(),
        },
    )
    .await?;
    let followers = if ann_follows_first {
        [&ann, &sam]
    } else {
        [&sam, &ann]
    };
    for who in followers {
        crate::follow::follow_issue(
            &g.world.ctx(),
            crate::follow::FollowIssue {
                session_token: who.session_token.clone(),
                issue: issue_id.to_string(),
            },
        )
        .await?;
        g.world.clock.advance(1);
    }
    crate::tags::change_label(
        &g.world.ctx(),
        crate::tags::ChangeLabel {
            session_token: g.admin.clone(),
            label: mango,
            version,
            name: Some("papaya".to_owned()),
            color: None,
            group: bb_core::own_fields::Edit::Keep,
            retired: None,
        },
    )
    .await?;
    // Another issue keeps a label of the group in THIRD, where only ann reads it: for her the group
    // is in use (the read the rule caches); for sam it is not.
    let kept = an_issue(&g.world, g.acme)
        .await
        .map_err(AppError::Internal)?;
    let (guava, _) = area_label(&g, "guava").await?;
    label_on(&g.world, &g.admin, kept, guava).await?;
    move_issue(
        &g.world.ctx(),
        MoveIssue {
            session_token: g.admin.clone(),
            issue: kept.to_string(),
            version: version_of(&g.world, &g.admin, kept).await?,
            project: "THIRD".to_owned(),
        },
    )
    .await?;
    // The label comes off again, so the group is in use on no issue the outsider reads.
    remove_label(
        &g.world.ctx(),
        RemoveLabel {
            session_token: g.admin.clone(),
            issue: issue_id.to_string(),
            label: mango,
        },
    )
    .await?;
    g.world
        .clock
        .advance(bb_core::notification::MAIL_BATCH_SECONDS + 1);
    // One round of the jobs: it holds both e-mails.
    g.world.ctx().run_due_jobs().await?;
    Ok((
        mails_to(&g.world, "ann@example.test"),
        mails_to(&g.world, "sam@example.test"),
    ))
}

#[tokio::test]
async fn sec_lm1_what_one_e_mail_of_a_round_may_name_never_reaches_the_next_one() -> Outcome {
    for ann_first in [true, false] {
        let (ann, sam) = two_recipients_in_one_round(ann_first).await?;
        assert!(ann.contains("papaya"), "ann knows the group: {ann}");
        assert!(!sam.contains("papaya") && !sam.contains("mango"), "{sam}");
    }
    Ok(())
}

#[tokio::test]
async fn sec_06_the_history_names_a_tag_only_where_the_reader_may_know_it() -> Outcome {
    let g = groups().await?;
    let sam = lead_of(&g.world, "sam@example.test", g.other).await?;
    let issue_id = an_issue(&g.world, g.acme)
        .await
        .map_err(AppError::Internal)?;
    // A tag limited to ACME is put on the issue and taken off again, and the issue moves to OTHER.
    let tag = super::scenarios_tags::tagged(&g.world, issue_id, "acme-only")
        .await
        .map_err(AppError::Internal)?;
    crate::tags::change_tag(
        &g.world.ctx(),
        crate::tags::ChangeTag {
            session_token: g.admin.clone(),
            tag,
            version: 1,
            name: None,
            color: None,
            projects: Some(vec![g.acme]),
        },
    )
    .await?;
    crate::tags::remove_tag(
        &g.world.ctx(),
        crate::tags::RemoveTag {
            session_token: g.admin.clone(),
            issue: issue_id.to_string(),
            tag,
        },
    )
    .await?;
    move_issue(
        &g.world.ctx(),
        MoveIssue {
            session_token: g.admin.clone(),
            issue: issue_id.to_string(),
            version: version_of(&g.world, &g.admin, issue_id).await?,
            project: "OTHER".to_owned(),
        },
    )
    .await?;
    let outside = read(&g.world, &sam.session_token, issue_id).await?;
    assert!(!outside.contains("acme-only"), "{outside}");
    // The administrator may know every tag, and reads it.
    let inside = read(&g.world, &g.admin, issue_id).await?;
    assert!(inside.contains("acme-only"), "{inside}");
    Ok(())
}
