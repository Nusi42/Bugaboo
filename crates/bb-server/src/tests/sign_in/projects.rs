//! Projects through the real wiring, on both databases (PRJ-01, PRJ-02, PRJ-03, PRIV-13, M2-F23):
//! nesting, short names and their history, what a person who may not see a project is told, the
//! preview of a move and the rule that a move lifts no *No access*.

use bb_app::access::{Permission, PersonRef};
use bb_app::auth::AppError;
use bb_app::projects::{
    ChangeProject, MovePreview, MovePreviewQuery, MoveProject, ProjectDetail, ProjectPage,
    ProjectQuery, ProjectView, ProjectsQuery,
};
use bb_app::roles::{AssignmentsQuery, Given, RemoveAssignment, Subject};
use bb_core::ErrorCode;
use bb_core::access::AppliesTo;
use bb_core::id::ProjectId;
use bb_core::sign_in::SecurityAction;
use bb_testing::each_database;

use super::policy_rig::{Member, Org};
use super::rig::{Outcome, Rig, refused};

/// What a lead of a project may do.
pub(super) const LEAD: [Permission; 5] = [
    Permission::SeeProjects,
    Permission::MakeSubProjects,
    Permission::MoveProjects,
    Permission::ChangeProjectSettings,
    Permission::DecideRoles,
];
/// What somebody who only looks may do.
pub(super) const VIEW: [Permission; 1] = [Permission::SeeProjects];

pub(super) fn api<'o>(org: &'o Org<'_>) -> &'o dyn bb_app::auth::AuthApi {
    org.rig.api()
}

pub(super) async fn detail(
    org: &Org<'_>,
    by: &Member,
    address: &str,
) -> Result<ProjectDetail, AppError> {
    Ok(api(org)
        .project(
            Rig::info(),
            ProjectQuery {
                session_token: by.session.clone(),
                address: address.to_owned(),
            },
        )
        .await?
        .into_value())
}

pub(super) async fn list(
    org: &Org<'_>,
    by: &Member,
    inside: Option<ProjectId>,
    search: Option<&str>,
) -> Result<ProjectPage, AppError> {
    Ok(api(org)
        .projects(
            Rig::info(),
            ProjectsQuery {
                session_token: by.session.clone(),
                inside,
                search: search.map(str::to_owned),
                outside: None,
                movable: None,
                cursor: None,
                limit: 100,
            },
        )
        .await?
        .into_value())
}

pub(super) async fn make(
    org: &Org<'_>,
    by: &Member,
    parent: Option<ProjectId>,
    short_name: &str,
) -> Result<ProjectView, AppError> {
    Ok(api(org)
        .make_project(
            Rig::info(),
            bb_app::projects::MakeProject {
                session_token: by.session.clone(),
                parent,
                short_name: short_name.to_owned(),
                name: format!("Project {short_name}"),
            },
        )
        .await?
        .into_value())
}

pub(super) async fn change(
    org: &Org<'_>,
    by: &Member,
    project: &ProjectView,
    name: Option<&str>,
    short_name: Option<&str>,
) -> Result<ProjectView, AppError> {
    Ok(api(org)
        .change_project(
            Rig::info(),
            ChangeProject {
                session_token: by.session.clone(),
                project: project.id,
                version: project.version,
                name: name.map(str::to_owned),
                short_name: short_name.map(str::to_owned),
            },
        )
        .await?
        .into_value())
}

pub(super) async fn preview(
    org: &Org<'_>,
    by: &Member,
    project: ProjectId,
    to: Option<ProjectId>,
) -> Result<MovePreview, AppError> {
    Ok(api(org)
        .preview_move(
            Rig::info(),
            MovePreviewQuery {
                session_token: by.session.clone(),
                project,
                to,
            },
        )
        .await?
        .into_value())
}

pub(super) async fn relocate(
    org: &Org<'_>,
    by: &Member,
    project: ProjectId,
    to: Option<ProjectId>,
    preview: &str,
) -> Result<ProjectView, AppError> {
    Ok(api(org)
        .move_project(
            Rig::info(),
            MoveProject {
                session_token: by.session.clone(),
                project,
                to,
                preview: preview.to_owned(),
            },
        )
        .await?
        .into_value())
}

/// The project as the person asks for it by address, without the answer's content: only whether
/// and how it was refused.
pub(super) async fn asked(org: &Org<'_>, by: &Member, address: &str) -> Option<AppError> {
    detail(org, by, address).await.err()
}

pub(super) fn missing_id() -> ProjectId {
    ProjectId::from_parts(jiff::Timestamp::UNIX_EPOCH, [9; 10])
        .unwrap_or_else(|_| unreachable!("the epoch is not before 1970"))
}

pub(super) fn names(page: &ProjectPage) -> Vec<String> {
    page.items
        .iter()
        .map(|item| item.short_name.to_string())
        .collect()
}

#[tokio::test]
async fn prj_01_projects_nest_to_any_depth_and_short_names_are_unique() -> Outcome {
    each_database("prj_01_nesting", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let mut parent = None;
        let mut made = Vec::new();
        // Twelve projects, each inside the one before.
        for number in 0..12 {
            let project = make(&org, &admin, parent, &format!("DEEP{number}")).await?;
            parent = Some(project.id);
            made.push(project);
        }
        let deepest = detail(&org, &admin, "DEEP11").await?;
        assert_eq!(deepest.trail.len(), 11);
        assert_eq!(deepest.trail[0].short_name.as_str(), "DEEP0");
        assert_eq!(deepest.project.parent, Some(made[10].id));
        // A short name is unique in the whole organization, at any depth.
        for place in [None, Some(made[3].id), Some(made[11].id)] {
            assert_eq!(
                refused(make(&org, &admin, place, "DEEP5").await),
                Some(ErrorCode::NameTaken)
            );
        }
        // The tree is walked one level at a time, and by search.
        assert_eq!(names(&list(&org, &admin, None, None).await?), ["DEEP0"]);
        let inside = list(&org, &admin, Some(made[4].id), None).await?;
        assert_eq!(names(&inside), ["DEEP5"]);
        assert!(inside.items[0].has_sub_projects);
        assert!(
            !list(&org, &admin, Some(made[11].id), None)
                .await?
                .items
                .iter()
                .any(|_| true)
        );
        // A search for nothing lists every project the person sees, wherever it sits.
        assert_eq!(list(&org, &admin, None, Some("")).await?.items.len(), 12);
        assert_eq!(
            names(&list(&org, &admin, None, Some("deep1")).await?),
            ["DEEP1", "DEEP10", "DEEP11"]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_01_a_short_name_breaks_no_rule_of_m2_q4_when_a_project_is_made() -> Outcome {
    each_database("prj_01_short_name_rules", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        // The server takes only capitals; the form turns typed letters into capitals.
        for bad in ["web", "W", "ABCDEFGHIJK", "2WEB", "WE B", "WEB-1", "ÄB", ""] {
            assert_eq!(
                refused(make(&org, &admin, None, bad).await),
                Some(ErrorCode::InvalidRequest),
                "{bad:?}"
            );
        }
        for good in ["WE", "APP2", "ABCDEFGHIJ"] {
            make(&org, &admin, None, good).await?;
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_01_an_old_short_name_redirects() -> Outcome {
    each_database("prj_01_old_short_name", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let web = make(&org, &admin, None, "WEB").await?;
        let renamed = change(&org, &admin, &web, None, Some("SITE")).await?;
        assert_eq!(renamed.short_name.as_str(), "SITE");
        assert_eq!(renamed.version, 2);
        // The old address still opens the project, and the answer holds the new short name.
        let old = detail(&org, &admin, "WEB").await?;
        assert_eq!(old.project.id, web.id);
        assert_eq!(old.project.short_name.as_str(), "SITE");
        assert_eq!(detail(&org, &admin, "SITE").await?.project.id, web.id);
        assert_eq!(
            detail(&org, &admin, &web.id.to_string()).await?.project.id,
            web.id
        );
        // Nobody else can take the old short name; the project can.
        assert_eq!(
            refused(make(&org, &admin, None, "WEB").await),
            Some(ErrorCode::NameTaken)
        );
        let other = make(&org, &admin, None, "OTHER").await?;
        assert_eq!(
            refused(change(&org, &admin, &other, None, Some("WEB")).await),
            Some(ErrorCode::NameTaken)
        );
        let back = change(&org, &admin, &renamed, None, Some("WEB")).await?;
        assert_eq!(back.short_name.as_str(), "WEB");
        // Now SITE is the old one, and stays with the project.
        assert_eq!(detail(&org, &admin, "SITE").await?.project.id, web.id);
        assert_eq!(
            refused(make(&org, &admin, None, "SITE").await),
            Some(ErrorCode::NameTaken)
        );
        // A change made on an old version is stopped, a name alone leaves the short name be.
        assert_eq!(
            refused(change(&org, &admin, &web, Some("Stale"), None).await),
            Some(ErrorCode::VersionChanged)
        );
        let named = change(&org, &admin, &back, Some("The website"), None).await?;
        assert_eq!(
            (named.name.as_str(), named.short_name.as_str()),
            ("The website", "WEB")
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_01_a_taken_short_name_is_told_only_to_people_who_may_make_projects() -> Outcome {
    each_database("prj_01_taken_short_name", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let secret = make(&org, &admin, None, "SECRET").await?;
        let renamed = change(&org, &admin, &secret, None, Some("OPEN")).await?;
        assert_eq!(renamed.short_name.as_str(), "OPEN");
        let ours = make(&org, &admin, None, "OURS").await?;

        // Maker may make projects at the top level and sees none of them.
        let maker_role = org.role("Maker", &[Permission::MakeProjects]).await?;
        let maker = org.person().await?;
        org.give_everywhere(&maker, maker_role.id).await?;
        // The same answer for a hidden project's short name, for its old one, and for a short
        // name that a project she cannot see once had; a free one works.
        let current = refused(make(&org, &maker, None, "OPEN").await);
        let old = refused(make(&org, &maker, None, "SECRET").await);
        assert_eq!(current, Some(ErrorCode::NameTaken));
        assert_eq!(old, current);
        assert_eq!(
            asked(&org, &maker, "OPEN").await,
            Some(AppError::Refused(ErrorCode::NotFound))
        );
        make(&org, &maker, None, "FREE").await?;

        // Cleo leads OURS: she may change its settings, but may make no project where it sits,
        // so she is told nothing about short names of others, taken or not, and never refused
        // differently for a taken one.
        let lead_role = org.role("Section lead", &LEAD).await?;
        let cleo = org.person().await?;
        org.give(&cleo, lead_role.id, ours.id).await?;
        let taken = refused(change(&org, &cleo, &ours, None, Some("OPEN")).await);
        let free = refused(change(&org, &cleo, &ours, None, Some("NOBODYS")).await);
        assert_eq!(taken, Some(ErrorCode::Forbidden));
        assert_eq!(free, taken);
        // Everything else about the project she may change.
        let named = change(&org, &cleo, &ours, Some("Ours, renamed"), None).await?;
        assert_eq!(named.name, "Ours, renamed");
        assert!(!detail(&org, &cleo, "OURS").await?.may_change_short_name);
        assert!(detail(&org, &admin, "OURS").await?.may_change_short_name);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_01_a_hidden_project_answers_not_found_like_a_missing_one() -> Outcome {
    each_database("prj_01_hidden_not_found", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let hidden = make(&org, &admin, None, "HIDDEN").await?;
        let inner = make(&org, &admin, Some(hidden.id), "INNER").await?;
        let bob = org.person().await?;
        let viewer = org.role("Looker", &VIEW).await?;
        let shown = make(&org, &admin, None, "SHOWN").await?;
        org.give(&bob, viewer.id, shown.id).await?;

        let not_found = Some(AppError::Refused(ErrorCode::NotFound));
        // By short name, by ID, and for what is inside a hidden project: the same as for a
        // project that does not exist.
        for address in [
            "HIDDEN",
            "INNER",
            &hidden.id.to_string(),
            &inner.id.to_string(),
        ] {
            assert_eq!(asked(&org, &bob, address).await, not_found, "{address}");
        }
        for address in [
            "MISSING",
            &missing_id().to_string(),
            "not an address",
            "hidden",
        ] {
            assert_eq!(asked(&org, &bob, address).await, not_found, "{address}");
        }
        // Changing, previewing, moving and looking inside answer the same.
        for project in [&hidden, &inner] {
            assert_eq!(
                refused(change(&org, &bob, project, Some("X"), None).await),
                Some(ErrorCode::NotFound)
            );
            assert_eq!(
                refused(preview(&org, &bob, project.id, None).await),
                Some(ErrorCode::NotFound)
            );
            assert_eq!(
                refused(relocate(&org, &bob, project.id, None, "x").await),
                Some(ErrorCode::NotFound)
            );
            assert_eq!(
                refused(list(&org, &bob, Some(project.id), None).await),
                Some(ErrorCode::NotFound)
            );
        }
        assert_eq!(
            refused(list(&org, &bob, Some(missing_id()), None).await),
            Some(ErrorCode::NotFound)
        );
        // He is told about no sub-project of one he may not see, and finds none by searching.
        assert_eq!(names(&list(&org, &bob, None, None).await?), ["SHOWN"]);
        assert!(
            list(&org, &bob, None, Some("inner"))
                .await?
                .items
                .is_empty()
        );
        // Putting a project into a hidden one is as if it did not exist, too.
        assert_eq!(
            refused(preview(&org, &bob, shown.id, Some(hidden.id)).await),
            refused(preview(&org, &bob, shown.id, Some(missing_id())).await)
        );
        // The same for somebody who is not signed in as anybody known.
        assert_eq!(
            refused(
                api(&org)
                    .project(
                        Rig::info(),
                        ProjectQuery {
                            session_token: "nothing".to_owned(),
                            address: "SHOWN".to_owned()
                        },
                    )
                    .await
            ),
            Some(ErrorCode::NotSignedIn)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_01_a_visible_sub_project_of_a_hidden_project_does_not_name_it() -> Outcome {
    each_database("prj_01_hidden_parent", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = make(&org, &admin, None, "ACME").await?;
        let web = make(&org, &admin, Some(acme.id), "WEB").await?;
        let api_project = make(&org, &admin, Some(web.id), "API").await?;
        let bea = org.person().await?;
        let viewer = org.role("Looker", &VIEW).await?;
        // Bea sees WEB and what is inside it, but not ACME above it.
        org.give(&bea, viewer.id, web.id).await?;

        // WEB is at the top of her tree; API inside it.
        let top = list(&org, &bea, None, None).await?;
        assert_eq!(names(&top), ["WEB"]);
        assert!(top.items[0].parent.is_none());
        assert!(top.items[0].has_sub_projects);
        assert_eq!(names(&list(&org, &bea, Some(web.id), None).await?), ["API"]);
        let found = detail(&org, &bea, "WEB").await?;
        assert!(found.project.parent.is_none());
        assert!(found.trail.is_empty());
        let deeper = detail(&org, &bea, "API").await?;
        assert_eq!(deeper.project.parent, Some(web.id));
        assert_eq!(
            deeper
                .trail
                .iter()
                .map(|step| step.short_name.as_str())
                .collect::<Vec<_>>(),
            ["WEB"]
        );
        // Nothing she is told names ACME: not in the JSON, not by name, short name or ID.
        let everything = serde_json::to_string(&(
            top,
            found,
            deeper,
            list(&org, &bea, None, Some("web")).await?,
        ))?;
        for hidden in ["ACME", &acme.id.to_string(), "Project ACME"] {
            assert!(!everything.contains(hidden), "{hidden} in {everything}");
        }
        // A change made by her names nothing above either.
        let changed = change(
            &org,
            &admin,
            &api_project,
            Some("Programming interface"),
            None,
        )
        .await?;
        assert_eq!(changed.parent, Some(web.id));
        // Seeing ACME and API but not WEB between them puts API at the top of her tree.
        let cleo = org.person().await?;
        org.give(&cleo, viewer.id, acme.id).await?;
        let no_access = org.forbid(&cleo, web.id).await;
        assert!(no_access.is_ok());
        org.assign(
            &org.admin,
            Subject::Person(cleo.id),
            Given::Role(viewer.id),
            Some(api_project.id),
            AppliesTo::Project,
        )
        .await?;
        assert_eq!(
            names(&list(&org, &cleo, None, None).await?),
            ["API", "ACME"]
        );
        assert!(detail(&org, &cleo, "API").await?.trail.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_03_the_move_preview_names_who_gains_and_loses_access() -> Outcome {
    each_database("prj_03_preview", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = make(&org, &admin, None, "ACME").await?;
        let other = make(&org, &admin, None, "OTHER").await?;
        let web = make(&org, &admin, Some(other.id), "WEB").await?;
        make(&org, &admin, Some(web.id), "API").await?;
        let viewer = org.role("Looker", &VIEW).await?;
        let bob = org.person().await?;
        let cleo = org.person().await?;
        org.give(&bob, viewer.id, acme.id).await?;
        org.give(&cleo, viewer.id, other.id).await?;

        // WEB, with API inside it, moves from OTHER into ACME.
        let shown = preview(&org, &admin, web.id, Some(acme.id)).await?;
        assert!(!shown.blocked && shown.lifts.is_empty() && !shown.truncated);
        assert_eq!(
            shown.to.as_ref().map(|to| to.short_name.as_str()),
            Some("ACME")
        );
        // Two people change, by name, because the administrator manages people.
        assert_eq!(shown.people.len(), 2);
        let line = |who: &Member| {
            shown
                .people
                .iter()
                .find(
                    |change| matches!(&change.person, PersonRef::Name { id, .. } if *id == who.id),
                )
                .ok_or("a person is not in the preview")
        };
        let gains = line(&bob)?;
        let mut at: Vec<&str> = gains
            .projects
            .iter()
            .map(|change| change.project.short_name.as_str())
            .collect();
        at.sort_unstable();
        assert_eq!(at, ["API", "WEB"]);
        assert!(
            gains
                .projects
                .iter()
                .all(|change| !change.now.sees && change.after.sees)
        );
        assert!(
            gains
                .projects
                .iter()
                .all(|change| change.after.roles == ["Looker"])
        );
        assert!(
            gains
                .projects
                .iter()
                .all(|change| change.gains.contains(&Permission::SeeProjects))
        );
        let loses = line(&cleo)?;
        assert_eq!(loses.projects.len(), 2);
        assert!(
            loses
                .projects
                .iter()
                .all(|change| change.now.sees && !change.after.sees)
        );
        assert!(
            loses
                .projects
                .iter()
                .all(|change| change.now.roles == ["Looker"] && change.after.roles.is_empty())
        );
        // Moving with the version of the preview does it; the project is inside ACME after.
        let moved = relocate(&org, &admin, web.id, Some(acme.id), &shown.preview).await?;
        assert_eq!(moved.parent, Some(acme.id));
        assert_eq!(moved.version, web.version + 1);
        assert_eq!(detail(&org, &bob, "API").await?.trail.len(), 2);
        assert!(
            asked(&org, &cleo, "API").await.is_some(),
            "Cleo no longer sees it"
        );
        // One event and one entry of the log, for the move.
        let logged = rig
            .database
            .security_log_entries(0, 10_000)
            .await?
            .iter()
            .filter(|entry| entry.action.as_str() == SecurityAction::ProjectMoved.code())
            .count();
        assert_eq!(logged, 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_03_a_project_cannot_move_below_itself() -> Outcome {
    each_database("prj_03_below_itself", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = make(&org, &admin, None, "ACME").await?;
        let web = make(&org, &admin, Some(acme.id), "WEB").await?;
        let api_project = make(&org, &admin, Some(web.id), "API").await?;
        for to in [acme.id, web.id, api_project.id] {
            assert_eq!(
                refused(preview(&org, &admin, acme.id, Some(to)).await),
                Some(ErrorCode::MoveIntoItself)
            );
            assert_eq!(
                refused(relocate(&org, &admin, acme.id, Some(to), "x").await),
                Some(ErrorCode::MoveIntoItself)
            );
        }
        // Where it is already is no move.
        assert_eq!(
            refused(preview(&org, &admin, web.id, Some(acme.id)).await),
            Some(ErrorCode::InvalidRequest)
        );
        // The projects above are where it may go; API can go to the top.
        let shown = preview(&org, &admin, api_project.id, None).await?;
        relocate(&org, &admin, api_project.id, None, &shown.preview).await?;
        assert!(detail(&org, &admin, "API").await?.trail.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_03_moving_needs_the_right_in_the_old_and_the_new_parent() -> Outcome {
    each_database("prj_03_both_parents", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = make(&org, &admin, None, "ACME").await?;
        let other = make(&org, &admin, None, "OTHER").await?;
        let web = make(&org, &admin, Some(acme.id), "WEB").await?;
        let lead = org.role("Section lead", &LEAD).await?;
        let viewer = org.role("Looker", &VIEW).await?;
        let bob = org.person().await?;
        // Bob leads ACME and only looks at OTHER.
        org.give(&bob, lead.id, acme.id).await?;
        org.give(&bob, viewer.id, other.id).await?;
        // Out of ACME, into OTHER: he may move in ACME, not in OTHER.
        assert_eq!(
            refused(preview(&org, &bob, web.id, Some(other.id)).await),
            Some(ErrorCode::Forbidden)
        );
        // To the top level counts as the organization, where he holds nothing.
        assert_eq!(
            refused(preview(&org, &bob, web.id, None).await),
            Some(ErrorCode::Forbidden)
        );
        // Now he leads OTHER too: both parents are his, and the move can be looked at and done.
        org.give(&bob, lead.id, other.id).await?;
        let shown = preview(&org, &bob, web.id, Some(other.id)).await?;
        relocate(&org, &bob, web.id, Some(other.id), &shown.preview).await?;
        assert_eq!(
            detail(&org, &bob, "WEB").await?.project.parent,
            Some(other.id)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_03_a_move_after_the_tree_changed_asks_again() -> Outcome {
    each_database("prj_03_stale_preview", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = make(&org, &admin, None, "ACME").await?;
        let web = make(&org, &admin, None, "WEB").await?;
        let viewer = org.role("Looker", &VIEW).await?;
        let bob = org.person().await?;
        let shown = preview(&org, &admin, web.id, Some(acme.id)).await?;
        assert!(shown.people.is_empty(), "nobody has a role yet");
        // Somebody gives Bob a role on ACME after the preview was made: the move would now change
        // what Bob sees, so the preview of the person who confirms is out of date.
        org.give(&bob, viewer.id, acme.id).await?;
        assert_eq!(
            refused(relocate(&org, &admin, web.id, Some(acme.id), &shown.preview).await),
            Some(ErrorCode::MovePreviewChanged)
        );
        // Nothing moved; looking again and confirming works.
        assert!(detail(&org, &admin, "WEB").await?.trail.is_empty());
        let again = preview(&org, &admin, web.id, Some(acme.id)).await?;
        assert_eq!(again.people.len(), 1);
        assert_ne!(again.preview, shown.preview);
        relocate(&org, &admin, web.id, Some(acme.id), &again.preview).await?;
        // The same goes for a rename of one of the two parents, and for an unknown version.
        let other = make(&org, &admin, None, "OTHER").await?;
        let third = preview(&org, &admin, web.id, Some(other.id)).await?;
        change(&org, &admin, &other, Some("Renamed"), None).await?;
        assert_eq!(
            refused(relocate(&org, &admin, web.id, Some(other.id), &third.preview).await),
            Some(ErrorCode::MovePreviewChanged)
        );
        assert_eq!(
            refused(relocate(&org, &admin, web.id, Some(other.id), "made up").await),
            Some(ErrorCode::MovePreviewChanged)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_03_a_move_cannot_let_somebody_lift_a_no_access_they_are_subject_to() -> Outcome {
    each_database("prj_03_lift", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = make(&org, &admin, None, "ACME").await?;
        let x = make(&org, &admin, None, "XRAY").await?;
        let inside = make(&org, &admin, Some(x.id), "XIN").await?;
        let lead = org.role("Section lead", &LEAD).await?;
        let bob = org.person().await?;
        // Bob leads ACME with its sub-projects, and has a No access on X.
        org.give(&bob, lead.id, acme.id).await?;
        let no_access = org.forbid(&bob, x.id).await?;
        assert!(!org.can(&bob, Permission::DecideRoles, Some(x.id)).await?);

        // The preview says who it would be, in words, and that the move is refused.
        let shown = preview(&org, &admin, x.id, Some(acme.id)).await?;
        assert!(shown.blocked);
        assert_eq!(shown.lifts.len(), 1);
        assert!(
            matches!(&shown.lifts[0].person, PersonRef::Name { name, .. } if name == "Bob Builder")
        );
        assert_eq!(shown.lifts[0].project.short_name.as_str(), "XRAY");
        // The move is refused, with the right preview and with a made up one.
        for version in [shown.preview.as_str(), "made up"] {
            assert_eq!(
                refused(relocate(&org, &admin, x.id, Some(acme.id), version).await),
                Some(ErrorCode::MoveLiftsNoAccess),
                "{version}"
            );
        }
        // Nothing moved, and Bob still cannot take the No access away or see X.
        assert!(detail(&org, &admin, "XRAY").await?.trail.is_empty());
        assert!(!org.can(&bob, Permission::DecideRoles, Some(x.id)).await?);
        let lifted = api(&org)
            .remove_assignment(
                Rig::info(),
                RemoveAssignment {
                    session_token: bob.session.clone(),
                    assignment: no_access.id,
                },
            )
            .await;
        assert!(matches!(
            refused(lifted),
            Some(ErrorCode::NotFound | ErrorCode::Forbidden)
        ));
        assert!(asked(&org, &bob, "XRAY").await.is_some());
        assert!(asked(&org, &bob, "XIN").await.is_some());

        // The same for a No access inside what moves, deeper down.
        let deep = org.person().await?;
        org.give(&deep, lead.id, acme.id).await?;
        org.forbid(&deep, inside.id).await?;
        let shown = preview(&org, &admin, x.id, Some(acme.id)).await?;
        assert_eq!(shown.lifts.len(), 2);
        assert_eq!(
            refused(relocate(&org, &admin, x.id, Some(acme.id), &shown.preview).await),
            Some(ErrorCode::MoveLiftsNoAccess)
        );

        // Take both No access rows away first: then the move is fine.
        for project in [x.id, inside.id] {
            take_no_access_away(&org, project).await?;
        }
        let shown = preview(&org, &admin, x.id, Some(acme.id)).await?;
        assert!(!shown.blocked);
        relocate(&org, &admin, x.id, Some(acme.id), &shown.preview).await?;
        Ok(())
    })
    .await
}

/// The administrator takes away every *No access* that is on the project itself.
async fn take_no_access_away(org: &Org<'_>, project: ProjectId) -> Outcome {
    let rows = api(org)
        .assignments(
            Rig::info(),
            AssignmentsQuery {
                session_token: org.admin.session.clone(),
                project: Some(project),
            },
        )
        .await?
        .into_value()
        .rows;
    for row in rows
        .iter()
        .filter(|row| row.role.is_none() && !row.inherited)
    {
        api(org)
            .remove_assignment(
                Rig::info(),
                RemoveAssignment {
                    session_token: org.admin.session.clone(),
                    assignment: row.id,
                },
            )
            .await?;
    }
    Ok(())
}

#[tokio::test]
async fn prj_03_a_move_out_from_under_a_no_access_is_named_in_the_preview() -> Outcome {
    each_database("prj_03_out_from_under", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = make(&org, &admin, None, "ACME").await?;
        let web = make(&org, &admin, Some(acme.id), "WEB").await?;
        let viewer = org.role("Looker", &VIEW).await?;
        let bob = org.person().await?;
        org.give_everywhere(&bob, viewer.id).await?;
        org.forbid(&bob, acme.id).await?;
        assert!(asked(&org, &bob, "WEB").await.is_some());
        let shown = preview(&org, &admin, web.id, None).await?;
        let change = &shown.people[0].projects[0];
        assert!(change.now.held_back && !change.after.held_back);
        assert!(!change.now.sees && change.after.sees);
        relocate(&org, &admin, web.id, None, &shown.preview).await?;
        assert!(asked(&org, &bob, "WEB").await.is_none(), "Bob now sees it");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_13_no_project_answer_holds_a_count() -> Outcome {
    each_database("priv_13_no_counts", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        // Two top-level projects, each with hidden sub-projects (three and none).
        let loud = make(&org, &admin, None, "LOUD").await?;
        let quiet = make(&org, &admin, None, "QUIET").await?;
        for short_name in ["ONE", "TWO", "THREE"] {
            make(&org, &admin, Some(loud.id), short_name).await?;
        }
        let viewer = org.role("Looker", &VIEW).await?;
        let bea = org.person().await?;
        // Bea sees the two projects themselves, and none of what is inside.
        for project in [&loud, &quiet] {
            org.assign(
                &org.admin,
                Subject::Person(bea.id),
                Given::Role(viewer.id),
                Some(project.id),
                AppliesTo::Project,
            )
            .await?;
        }
        let top = list(&org, &bea, None, None).await?;
        assert_eq!(names(&top), ["LOUD", "QUIET"]);
        // The projects that hold hidden sub-projects look exactly like the one that holds none.
        assert!(top.items.iter().all(|item| !item.has_sub_projects));
        assert!(
            list(&org, &bea, Some(loud.id), None)
                .await?
                .items
                .is_empty()
        );
        let (a, b) = (
            detail(&org, &bea, "LOUD").await?,
            detail(&org, &bea, "QUIET").await?,
        );
        assert_eq!(a.permissions, b.permissions);
        assert_eq!(a.trail, b.trail);

        // No property of any answer is a count of anything, and the administrator's answers
        // hold none either: `has_sub_projects` is a yes or no.
        let answers = [
            serde_json::to_value(&top)?,
            serde_json::to_value(&a)?,
            serde_json::to_value(preview(&org, &admin, quiet.id, Some(loud.id)).await?)?,
            serde_json::to_value(list(&org, &admin, None, None).await?)?,
            serde_json::to_value(&loud)?,
        ];
        for answer in &answers {
            assert_no_count(answer, "");
        }
        Ok(())
    })
    .await
}

/// No member of the JSON is named like a count, and no member holds a number except the version
/// of a project (a number of changes made to that one project, which counts nothing hidden).
fn assert_no_count(value: &serde_json::Value, path: &str) {
    match value {
        serde_json::Value::Object(members) => {
            for (name, inner) in members {
                let at = format!("{path}/{name}");
                for counter in [
                    "count",
                    "total",
                    "number",
                    "size",
                    "length",
                    "sub_projects_count",
                ] {
                    assert!(!name.contains(counter), "{at} looks like a count");
                }
                if inner.is_number() {
                    assert_eq!(name, "version", "{at} is a number");
                }
                assert_no_count(inner, &at);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                assert_no_count(item, path);
            }
        }
        _ => {}
    }
}

#[tokio::test]
async fn prj_01_changes_write_their_event_and_entry_of_the_log() -> Outcome {
    each_database("prj_01_event_and_log", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let web = make(&org, &admin, None, "WEB").await?;
        let entries = |action: SecurityAction| {
            let database = rig.database.clone();
            async move {
                Ok::<_, bb_store::StoreError>(
                    database
                        .security_log_entries(0, 10_000)
                        .await?
                        .iter()
                        .filter(|entry| entry.action.as_str() == action.code())
                        .count(),
                )
            }
        };
        let renamed = change(&org, &admin, &web, Some("The website"), None).await?;
        assert_eq!(entries(SecurityAction::ProjectRenamed).await?, 1);
        change(&org, &admin, &renamed, None, Some("SITE")).await?;
        assert_eq!(entries(SecurityAction::ProjectShortNameChanged).await?, 1);
        // A refused change writes nothing.
        let before = rig.database.security_log_entries(0, 10_000).await?.len();
        assert!(
            change(&org, &admin, &web, Some("Stale"), None)
                .await
                .is_err()
        );
        assert_eq!(
            rig.database.security_log_entries(0, 10_000).await?.len(),
            before
        );
        Ok(())
    })
    .await
}
