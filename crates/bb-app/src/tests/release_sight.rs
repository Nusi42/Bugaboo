//! What a reader is told of a release they may not know (ISS-08, ADR-0049, decisions 1, 7 and 8): a
//! release of a project they do not see, or of a project kept from AI, is in no preview of a tool,
//! in no answer to the AI, and a change of theirs neither takes it away nor reorders it.

use bb_core::given::SetTo;
use bb_core::id::{IssueId, ReleaseId};
use bb_policy::StartingRole;
use serde_json::{Value, json};

use super::ai_tools::{call, mark};
use super::releases::{a_release, a_sub_project, previewing, seen, set_releases};
use super::scenarios::{a_project, text, token};
use super::scenarios_issues::an_issue;
use super::scenarios_searches::with_role;
use super::world::{Outcome, World};
use crate::ai::NeverSentTarget;
use crate::ai::tools::call_tool;
use crate::auth::Started;
use crate::issues::{ChangeIssue, IssueView, MoveIssue, change_issue, move_issue};

/// A project APP with a sub-project WEB, a release of each, an issue of WEB that Ada put into both
/// fields with the release of APP, and Tea, who sees only WEB.
struct Split {
    world: World,
    issue: IssueId,
    top: ReleaseId,
    web: ReleaseId,
    tea: Started,
}

async fn split() -> Result<Split, String> {
    let world = World::new().await?;
    let app = a_project(&world, "APP").await?;
    let web_project = a_sub_project(&world, app, "WEB").await?;
    let top = a_release(&world, "APP", "Moonbeam").await?.id;
    let web = a_release(&world, "WEB", "2.4").await?.id;
    let issue = an_issue(&world, web_project).await?;
    set_releases(&world, issue, Some(vec![top]), Some(SetTo::Value(top)))
        .await
        .map_err(text)?;
    let tea = world.person("tea@example.test").await.map_err(text)?;
    with_role(&world, &tea, StartingRole::TeamMember, Some(web_project)).await?;
    Ok(Split {
        world,
        issue,
        top,
        web,
        tea,
    })
}

/// Changes the releases of the issue as Tea.
async fn as_tea(
    split: &Split,
    found: Option<Vec<ReleaseId>>,
    planned: Option<SetTo<ReleaseId>>,
) -> Result<IssueView, String> {
    let version = seen(&split.world, split.issue).await?.version;
    Ok(change_issue::<_, IssueView>(
        &split.world.ctx(),
        ChangeIssue {
            found_in_release: found,
            planned_for_release: planned,
            ..ChangeIssue::of(&split.tea.session_token, split.issue.to_string(), version)
        },
    )
    .await
    .map_err(text)?
    .into_value())
}

#[tokio::test]
async fn iss_08_a_tool_preview_tells_nothing_of_a_release_the_reader_may_not_see() -> Outcome {
    let split = split().await?;
    let version = seen(&split.world, split.issue).await?.version;
    let preview = call_tool(
        &split.world.ctx(),
        previewing(
            &split.tea.session_token,
            "update_issue",
            json!({
                "issue": split.issue.to_string(),
                "version": version,
                "found_in_release": [split.web.to_string()],
                "planned_for_release": split.web.to_string(),
            }),
        ),
    )
    .await
    .map_err(text)?
    .into_value();
    let said = serde_json::to_string(&preview).map_err(text)?;
    assert!(!said.contains("Moonbeam"), "{said}");
    assert!(!said.contains(&split.top.to_string()), "{said}");
    // Both fields change for her, from nothing she sees to the release she named.
    let fields: Vec<&str> = preview
        .changes
        .iter()
        .map(|change| change.field.as_str())
        .collect();
    assert_eq!(fields, ["found-in-release", "planned-for-release"]);
    let found = preview
        .changes
        .iter()
        .find(|change| change.field == "found-in-release")
        .ok_or("no change of found in release")?;
    assert_eq!(found.before, Some(json!([])));
    // A change she makes of nothing she sees is no change to tell.
    let nothing = call_tool(
        &split.world.ctx(),
        previewing(
            &split.tea.session_token,
            "update_issue",
            json!({"issue": split.issue.to_string(), "version": version, "found_in_release": []}),
        ),
    )
    .await
    .map_err(text)?
    .into_value();
    assert!(nothing.changes.is_empty(), "{:?}", nothing.changes);
    Ok(())
}

#[tokio::test]
async fn iss_08_a_change_keeps_a_release_the_person_may_not_see_and_its_place() -> Outcome {
    let split = split().await?;
    // She sees neither field's release, and her list keeps it first.
    let view = as_tea(&split, Some(vec![split.web]), Some(SetTo::Nothing)).await?;
    assert_eq!(view.found_in_release.clone().unwrap_or_default().len(), 1);
    assert!(view.planned_for_release.is_none());
    let ada = seen(&split.world, split.issue).await?;
    let ids = |view: &IssueView| -> Vec<ReleaseId> {
        view.found_in_release
            .clone()
            .unwrap_or_default()
            .iter()
            .map(|each| each.id)
            .collect()
    };
    assert_eq!(ids(&ada), [split.top, split.web]);
    // Setting no planned release kept the one she cannot see.
    assert_eq!(ada.planned_for_release.map(|each| each.id), Some(split.top));
    // Naming one she sees takes its place.
    as_tea(&split, None, Some(SetTo::Value(split.web))).await?;
    let ada = seen(&split.world, split.issue).await?;
    assert_eq!(ada.planned_for_release.map(|each| each.id), Some(split.web));
    // Taking hers away leaves the hidden one where it was.
    as_tea(&split, Some(Vec::new()), None).await?;
    assert_eq!(ids(&seen(&split.world, split.issue).await?), [split.top]);
    Ok(())
}

/// The names of the releases in what a tool answered.
fn told(answer: &Value) -> String {
    answer.to_string()
}

/// An issue of APP that names, in both fields, a release of SEC, a project kept from AI.
async fn kept_world() -> Result<(World, IssueId, ReleaseId), String> {
    let world = World::new().await?;
    let sec = a_project(&world, "SEC").await?;
    a_project(&world, "APP").await?;
    let secret = a_release(&world, "SEC", "Classified").await?.id;
    let issue = an_issue(&world, sec).await?;
    set_releases(
        &world,
        issue,
        Some(vec![secret]),
        Some(SetTo::Value(secret)),
    )
    .await
    .map_err(text)?;
    let version = seen(&world, issue).await?.version;
    move_issue(
        &world.ctx(),
        MoveIssue {
            session_token: token(&world),
            issue: issue.to_string(),
            version,
            project: "APP".to_owned(),
        },
    )
    .await
    .map_err(text)?;
    mark(&world, NeverSentTarget::Project("SEC".to_owned()))
        .await
        .map_err(|error| error.to_string())?;
    Ok((world, issue, secret))
}

/// The AI sets *Found in release* (and *Planned for release* when given) of the issue, for real.
async fn ai_sets(
    world: &World,
    issue: IssueId,
    found: &[ReleaseId],
    planned: Option<&str>,
) -> Result<crate::ai::tools::ToolAnswer, crate::auth::AppError> {
    let version = seen(world, issue)
        .await
        .map_err(crate::auth::AppError::Internal)?
        .version;
    let mut input = json!({
        "issue": issue.to_string(),
        "version": version,
        "found_in_release": found.iter().map(ToString::to_string).collect::<Vec<_>>(),
    });
    if let Some(planned) = planned {
        input["planned_for_release"] = json!(planned);
    }
    Ok(call_tool(
        &world.ctx(),
        crate::ai::tools::ToolCallQuery {
            mode: crate::ai::tools::ToolMode::Apply,
            ..call(&token(world), "update_issue", input)
        },
    )
    .await?
    .into_value())
}

#[tokio::test]
async fn iss_08_the_ai_is_told_nothing_of_a_release_of_a_project_kept_from_it() -> Outcome {
    let (world, issue, secret) = kept_world().await?;
    // The issue as the AI reads it names neither field's release.
    let read = call_tool(
        &world.ctx(),
        call(
            &token(&world),
            "get_issue",
            json!({"issue": issue.to_string()}),
        ),
    )
    .await
    .map_err(text)?
    .into_value();
    let said = told(&serde_json::to_value(&read).map_err(text)?);
    assert!(!said.contains("Classified"), "{said}");
    // The releases of that project are not there for the AI.
    let listed = call_tool(
        &world.ctx(),
        call(&token(&world), "list_releases", json!({"project": "SEC"})),
    )
    .await;
    assert!(
        matches!(
            listed,
            Err(crate::auth::AppError::Refused(bb_core::ErrorCode::NotFound))
        ),
        "{listed:?}"
    );
    // A change the AI proposes neither shows nor takes away what it may not know.
    let version = seen(&world, issue).await?.version;
    let preview = call_tool(
        &world.ctx(),
        previewing(
            &token(&world),
            "update_issue",
            json!({
                "issue": issue.to_string(),
                "version": version,
                "found_in_release": [],
                "planned_for_release": "",
            }),
        ),
    )
    .await
    .map_err(text)?
    .into_value();
    assert!(preview.changes.is_empty(), "{:?}", preview.changes);
    // Made for real, the change keeps the release kept from AI in both fields at its place, and
    // the one the AI named comes after it.
    let open = a_release(&world, "APP", "1.0").await?.id;
    ai_sets(&world, issue, &[open], Some(""))
        .await
        .map_err(text)?;
    let still = seen(&world, issue).await?;
    let found: Vec<ReleaseId> = still
        .found_in_release
        .unwrap_or_default()
        .iter()
        .map(|each| each.id)
        .collect();
    assert_eq!(found, [secret, open]);
    assert_eq!(still.planned_for_release.map(|each| each.id), Some(secret));
    Ok(())
}

#[tokio::test]
async fn iss_08_an_agent_proposing_a_release_is_shown_it_only_when_it_may_know_it() -> Outcome {
    use super::agent_tools::previewed;
    use super::agent_work_rules::{found_by_search, made, rule_world};
    let world = World::with_ai_apps_and_agents().await?;
    let given = rule_world(&world).await?;
    let rule = made(&world, &given).await?;
    a_project(&world, "SEC").await?;
    let open = a_release(&world, "WEB", "2.4").await?.id;
    let secret = a_release(&world, "SEC", "Classified").await?.id;
    mark(&world, NeverSentTarget::Project("SEC".to_owned())).await?;
    found_by_search(&world);
    world
        .ctx()
        .run_rule_matches(&json!({ "project": given.project.to_string() }))
        .await?;
    let proposing = |release: ReleaseId| {
        json!({
            "rule": rule.id.to_string(),
            "issue": given.issue.to_string(),
            "field": "found-in-release",
            "value": release.to_string(),
        })
    };
    // The release the agent may know is in the preview, by its name.
    let shown = previewed(&world, &given.key, "propose", proposing(open)).await?;
    let change = shown
        .changes
        .iter()
        .find(|change| change.field == "found-in-release")
        .ok_or("no change of found in release")?;
    assert_eq!(
        change
            .after
            .as_ref()
            .and_then(|after| after[0]["name"].as_str()),
        Some("2.4")
    );
    // One of a project kept from AI is no change it is told of.
    let kept = previewed(&world, &given.key, "propose", proposing(secret)).await?;
    assert!(kept.changes.is_empty(), "{:?}", kept.changes);
    let said = serde_json::to_string(&kept).map_err(text)?;
    assert!(!said.contains("Classified"), "{said}");
    // One in the trash is no change either, like an unknown ID.
    let trashed = a_release(&world, "WEB", "2.9").await?.id;
    crate::releases::delete_release(
        &world.ctx(),
        super::releases::deleting(&token(&world), "WEB", trashed),
    )
    .await?;
    let gone = previewed(&world, &given.key, "propose", proposing(trashed)).await?;
    assert!(gone.changes.is_empty(), "{:?}", gone.changes);
    Ok(())
}

/// The releases the last search the store was asked for read as the ones the reader may know.
fn known_by_last_search(world: &World) -> Result<Option<Vec<ReleaseId>>, String> {
    let request = world.store.lock().searches.pop().ok_or("no search ran")?;
    Ok(request.releases_known.map(|mut known| {
        known.sort_unstable();
        known
    }))
}

#[tokio::test]
async fn iss_08_a_search_reads_the_releases_known_only_when_it_names_a_release_field() -> Outcome {
    use crate::search::{
        SearchCountQuery, SearchGroupsQuery, SearchScope, search_count, search_groups,
    };
    let split = split().await?;
    let counted = |text: &str| SearchCountQuery {
        session_token: split.tea.session_token.clone(),
        text: text.to_owned(),
        scope: SearchScope::Everywhere,
    };
    // A search that names no release field reads none.
    search_count(&split.world.ctx(), counted("title contains crash"))
        .await
        .map_err(text)?;
    assert_eq!(known_by_last_search(&split.world)?, None);
    // In the filter, in the order and in the groups: only the releases of the projects she sees.
    for asked in ["planned for release is empty", "sort by found in release"] {
        search_count(&split.world.ctx(), counted(asked))
            .await
            .map_err(text)?;
        assert_eq!(
            known_by_last_search(&split.world)?,
            Some(vec![split.web]),
            "{asked}"
        );
    }
    search_groups(
        &split.world.ctx(),
        SearchGroupsQuery {
            session_token: split.tea.session_token.clone(),
            text: String::new(),
            scope: SearchScope::Everywhere,
            by: "planned for release".to_owned(),
        },
    )
    .await
    .map_err(text)?;
    assert_eq!(known_by_last_search(&split.world)?, Some(vec![split.web]));
    // Grouped by another field, none again.
    search_groups(
        &split.world.ctx(),
        SearchGroupsQuery {
            session_token: split.tea.session_token.clone(),
            text: String::new(),
            scope: SearchScope::Everywhere,
            by: "priority".to_owned(),
        },
    )
    .await
    .map_err(text)?;
    assert_eq!(known_by_last_search(&split.world)?, None);
    Ok(())
}

#[tokio::test]
async fn iss_08_a_search_of_the_ai_knows_no_release_of_a_project_kept_from_it() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "SEC").await?;
    a_project(&world, "APP").await?;
    a_release(&world, "SEC", "Classified").await?;
    let open = a_release(&world, "APP", "1.0").await?.id;
    mark(&world, NeverSentTarget::Project("SEC".to_owned())).await?;
    call_tool(
        &world.ctx(),
        call(
            &token(&world),
            "search_issues",
            json!({"query": "found in release is empty"}),
        ),
    )
    .await
    .map_err(text)?;
    assert_eq!(known_by_last_search(&world)?, Some(vec![open]));
    Ok(())
}

#[tokio::test]
async fn iss_08_the_ai_keeps_20_releases_beside_one_kept_from_it() -> Outcome {
    let (world, issue, secret) = kept_world().await?;
    let mut open = Vec::new();
    for at in 0..bb_core::project_release::FOUND_IN_RELEASES_MAX {
        open.push(a_release(&world, "APP", &format!("1.{at}")).await?.id);
    }
    // The one kept from AI counts toward no limit: 20 the AI may know are not too many.
    ai_sets(&world, issue, &open, None).await.map_err(text)?;
    let found: Vec<ReleaseId> = seen(&world, issue)
        .await?
        .found_in_release
        .unwrap_or_default()
        .iter()
        .map(|each| each.id)
        .collect();
    assert_eq!(found.len(), open.len() + 1);
    assert_eq!(found[0], secret);
    Ok(())
}

#[tokio::test]
async fn iss_08_the_ai_naming_a_release_kept_from_it_is_refused_like_an_unknown_one() -> Outcome {
    let (world, issue, _) = kept_world().await?;
    let other = a_release(&world, "SEC", "Hidden").await?.id;
    let unknown = ReleaseId::generate(&mut world.ctx().ids()).map_err(text)?;
    for release in [other, unknown] {
        let refused = ai_sets(&world, issue, &[release], None).await;
        assert!(
            matches!(
                refused,
                Err(crate::auth::AppError::Refused(
                    bb_core::ErrorCode::ReleaseNotOffered
                ))
            ),
            "{refused:?}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn iss_08_a_search_reads_the_releases_known_once_for_names_ids_and_groups() -> Outcome {
    use crate::search::{SearchGroupsQuery, SearchScope, search_groups};
    use std::sync::atomic::Ordering;
    let split = split().await?;
    let reads = || {
        split
            .world
            .store
            .lock()
            .state
            .release_list_reads
            .load(Ordering::Relaxed)
    };
    let before = reads();
    // A name in the filter, the IDs a release field reads, and the names of the groups.
    search_groups(
        &split.world.ctx(),
        SearchGroupsQuery {
            session_token: split.tea.session_token.clone(),
            text: "found in release is \"2.4\"".to_owned(),
            scope: SearchScope::Everywhere,
            by: "planned for release".to_owned(),
        },
    )
    .await
    .map_err(text)?;
    assert_eq!(reads() - before, 1);
    // A search that names no release reads none.
    let before = reads();
    search_groups(
        &split.world.ctx(),
        SearchGroupsQuery {
            session_token: split.tea.session_token.clone(),
            text: String::new(),
            scope: SearchScope::Everywhere,
            by: "priority".to_owned(),
        },
    )
    .await
    .map_err(text)?;
    assert_eq!(reads(), before);
    Ok(())
}

#[tokio::test]
async fn iss_08_the_ai_totals_by_release_name_no_release_kept_from_it() -> Outcome {
    use crate::search::{GroupKey, SearchRequest};
    let (world, _, secret) = kept_world().await?;
    let open = a_release(&world, "APP", "1.0").await?.id;
    // What the statement would count: one issue for each release, one without.
    world.store.lock().search_totals = super::fake_search::counted(vec![
        (GroupKey::Release(open), 1),
        (GroupKey::Release(secret), 1),
        (GroupKey::Empty, 1),
    ]);
    let answer = call_tool(
        &world.ctx(),
        call(
            &token(&world),
            "search_totals",
            json!({"query": "", "group_by": "planned for release"}),
        ),
    )
    .await
    .map_err(text)?
    .into_value();
    let said = serde_json::to_string(&answer).map_err(text)?;
    // The release the AI may know is a group by its name; the one kept from it is no group and no
    // name, and its issue counts with none.
    assert!(said.contains("1.0"), "{said}");
    assert!(!said.contains("Classified"), "{said}");
    assert!(!said.contains(&secret.to_string()), "{said}");
    let request: SearchRequest = world.store.lock().searches.pop().ok_or("no search ran")?;
    assert_eq!(request.releases_known, Some(vec![open]));
    let grouped = world
        .store
        .lock()
        .totals_asked
        .last()
        .and_then(|asked| asked.by.clone())
        .map(|by| by.word);
    assert_eq!(grouped.as_deref(), Some("planned for release"));
    // Two groups: the release by its name with its issue, and none with the other two.
    assert_eq!(said.matches("        issues: ").count(), 2, "{said}");
    // The fields of a group may come in any order, so the group of none is the one whose
    // key is empty.
    let none = said
        .split("      group:")
        .find(|group| group.contains("type: empty"))
        .ok_or("no group of none")?;
    assert!(none.contains("issues: 2"), "{said}");
    Ok(())
}

#[tokio::test]
async fn iss_08_a_board_and_a_calendar_of_a_search_by_release_answer_as_any_other() -> Outcome {
    use super::scenarios_searches::save;
    use crate::boards::{BoardQuery, board};
    use crate::calendar::{CalendarQuery, calendar};
    use crate::search::ViewSearch;
    let split = split().await?;
    for (name, text) in [
        ("Planned", "planned for release is \"2.4\""),
        ("By release", "sort by found in release"),
    ] {
        let saved = save(&split.world, &split.tea.session_token, name, text)
            .await
            .map_err(super::scenarios::text)?
            .id;
        let shown = board(
            &split.world.ctx(),
            BoardQuery {
                session_token: split.tea.session_token.clone(),
                search: ViewSearch::Saved(saved),
                project: None,
                only_this_project: false,
                options: None,
                more: None,
                limit: 10,
            },
        )
        .await;
        assert!(shown.is_ok(), "{text}: {:?}", shown.err());
        let days = calendar(
            &split.world.ctx(),
            CalendarQuery {
                session_token: split.tea.session_token.clone(),
                search: ViewSearch::Saved(saved),
                project: None,
                only_this_project: false,
                options: bb_core::saved_search::CalendarOptions::default(),
                first: jiff::civil::date(2027, 3, 1),
                last: jiff::civil::date(2027, 3, 31),
            },
        )
        .await;
        assert!(days.is_ok(), "{text}: {:?}", days.err());
        // The search the view ran read the releases Tea may know, so no release field went unread.
        let request = split
            .world
            .store
            .lock()
            .searches
            .pop()
            .ok_or("no search ran")?;
        assert_eq!(request.releases_known, Some(vec![split.web]), "{text}");
    }
    // A text that names a release Tea may not know is refused like an unknown name.
    let unknown = board(
        &split.world.ctx(),
        BoardQuery {
            session_token: split.tea.session_token.clone(),
            search: ViewSearch::Text("planned for release is \"Moonbeam\"".to_owned()),
            project: None,
            only_this_project: false,
            options: None,
            more: None,
            limit: 10,
        },
    )
    .await;
    assert!(
        matches!(unknown, Err(crate::auth::AppError::Mistakes(_))),
        "{:?}",
        unknown.err()
    );
    Ok(())
}
