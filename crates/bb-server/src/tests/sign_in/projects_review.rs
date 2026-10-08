//! Probes from the review of WP-08: each one is a way around a rule of the project use cases that
//! must stay shut. Every one runs on both databases (ARCH-DB-PARITY).

use bb_app::access::Permission;
use bb_app::projects::{MoveRight, ProjectPage, ProjectsQuery};
use bb_core::ErrorCode;
use bb_core::id::ProjectId;
use bb_testing::{TestResult, each_database};

use super::policy_rig::{Member, Org};
use super::projects::{
    LEAD, VIEW, api, asked, detail, list, make, missing_id, names, preview, relocate,
};
use super::rig::{Rig, refused};

/// The most levels a tree of projects has.
const LEVELS: usize = bb_core::project::DEPTH_MAX;

/// The projects the person sees, leaving out a project and what is inside it.
async fn list_outside(
    org: &Org<'_>,
    by: &Member,
    outside: Option<ProjectId>,
) -> Result<ProjectPage, bb_app::auth::AppError> {
    list_places(org, by, outside, None).await
}

/// The projects the person sees, filtered as the place picker of a move asks.
async fn list_places(
    org: &Org<'_>,
    by: &Member,
    outside: Option<ProjectId>,
    movable: Option<ProjectId>,
) -> Result<ProjectPage, bb_app::auth::AppError> {
    Ok(api(org)
        .projects(
            Rig::info(),
            ProjectsQuery {
                session_token: by.session.clone(),
                inside: None,
                search: None,
                outside,
                movable,
                cursor: None,
                limit: 100,
            },
        )
        .await?
        .into_value())
}

#[tokio::test]
async fn prj_02_leaving_a_project_out_names_no_hidden_project() -> TestResult {
    each_database("prj_02_outside_hidden", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = make(&org, &admin, None, "ACME").await?;
        let web = make(&org, &admin, Some(acme.id), "WEB").await?;
        let looker = org.role("Looker", &VIEW).await?;
        let bea = org.person().await?;
        org.give(&bea, looker.id, web.id).await?;
        // WEB is at the top of her tree. Leaving out the hidden project above it, or one that
        // does not exist, changes nothing: the two answer alike.
        let plain = names(&list_outside(&org, &bea, None).await?);
        assert_eq!(plain, ["WEB"]);
        let hidden = list_outside(&org, &bea, Some(acme.id)).await?;
        let missing = list_outside(&org, &bea, Some(missing_id())).await?;
        assert_eq!(names(&hidden), plain);
        assert_eq!(names(&missing), plain);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_01_projects_are_nested_to_the_limit_and_no_further() -> TestResult {
    each_database("prj_01_depth_limit", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let mut chain: Vec<ProjectId> = Vec::new();
        for level in 0..LEVELS {
            let parent = chain.last().copied();
            chain.push(make(&org, &admin, parent, &format!("LVL{level}")).await?.id);
        }
        // One more level is refused with a sentence, not made and not an internal failure.
        let deepest = *chain.last().ok_or("no projects")?;
        let too_deep = refused(make(&org, &admin, Some(deepest), "ONEMORE").await);
        assert_eq!(too_deep, Some(ErrorCode::ProjectTooDeep));
        // The deepest project answers like every other, to those who see it and to those who do
        // not, and a list and a GET agree.
        let looker = org.role("Looker", &VIEW).await?;
        let bea = org.person().await?;
        org.give(&bea, looker.id, deepest).await?;
        let address = format!("LVL{}", LEVELS - 1);
        assert_eq!(
            names(&list(&org, &bea, None, None).await?),
            std::slice::from_ref(&address)
        );
        assert_eq!(detail(&org, &bea, &address).await?.project.id, deepest);
        let bob = org.person().await?;
        let hidden = asked(&org, &bob, &address).await;
        assert_eq!(hidden, asked(&org, &bob, "NOSUCH").await);
        assert!(hidden.is_some());
        // A move that would push what is inside past the limit is refused, one that fits is not.
        let x = make(&org, &admin, None, "XROOT").await?;
        make(&org, &admin, Some(x.id), "XLEAF").await?;
        let one_too_deep = chain[LEVELS - 2];
        assert_eq!(
            refused(preview(&org, &admin, x.id, Some(one_too_deep)).await),
            Some(ErrorCode::ProjectTooDeep)
        );
        let fits = chain[LEVELS - 3];
        let shown = preview(&org, &admin, x.id, Some(fits)).await?;
        relocate(&org, &admin, x.id, Some(fits), &shown.preview).await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_03_moving_needs_move_at_the_old_parent_also_with_move_at_the_new_one() -> TestResult {
    each_database("prj_03_old_parent_needs_move", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = make(&org, &admin, None, "ACME").await?;
        let other = make(&org, &admin, None, "OTHER").await?;
        let web = make(&org, &admin, Some(acme.id), "WEB").await?;
        let lead = org.role("Section lead", &LEAD).await?;
        let looker = org.role("Looker", &VIEW).await?;
        let cleo = org.person().await?;
        // Cleo only looks at ACME, where WEB is, and leads OTHER, where it would go.
        org.give(&cleo, looker.id, acme.id).await?;
        org.give(&cleo, lead.id, other.id).await?;
        assert_eq!(
            refused(preview(&org, &cleo, web.id, Some(other.id)).await),
            Some(ErrorCode::Forbidden)
        );
        assert_eq!(
            refused(relocate(&org, &cleo, web.id, Some(other.id), "x").await),
            Some(ErrorCode::Forbidden)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_03_a_visible_project_under_a_hidden_parent_is_forbidden_to_move_not_missing()
-> TestResult {
    each_database("prj_03_hidden_parent_move", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = make(&org, &admin, None, "ACME").await?;
        let web = make(&org, &admin, Some(acme.id), "WEB").await?;
        let other = make(&org, &admin, None, "OTHER").await?;
        let lead = org.role("Section lead", &LEAD).await?;
        let bea = org.person().await?;
        // Bea leads WEB and OTHER but does not see ACME, which holds WEB. Whatever the place,
        // she is told she may not: the same as for a parent she can see, and no hint of it.
        org.give(&bea, lead.id, web.id).await?;
        org.give(&bea, lead.id, other.id).await?;
        for to in [Some(other.id), None] {
            assert_eq!(
                refused(preview(&org, &bea, web.id, to).await),
                Some(ErrorCode::Forbidden)
            );
            assert_eq!(
                refused(relocate(&org, &bea, web.id, to, "x").await),
                Some(ErrorCode::Forbidden)
            );
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_03_a_project_says_whether_the_person_may_move_it() -> TestResult {
    each_database("prj_03_may_move_flag", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = make(&org, &admin, None, "ACME").await?;
        let web = make(&org, &admin, Some(acme.id), "WEB").await?;
        let lead = org.role("Section lead", &LEAD).await?;
        let looker = org.role("Looker", &VIEW).await?;
        let (ana, bea, cleo) = (
            org.person().await?,
            org.person().await?,
            org.person().await?,
        );
        // Ana leads ACME (so moves WEB), Bea only leads WEB (Move there is not Move in ACME),
        // Cleo only looks.
        org.give(&ana, lead.id, acme.id).await?;
        org.give(&bea, lead.id, web.id).await?;
        org.give(&cleo, looker.id, acme.id).await?;
        let mut flags = Vec::new();
        for who in [&admin, &ana, &bea, &cleo] {
            flags.push(detail(&org, who, "WEB").await?.may_move);
        }
        // The top level is offered where the project is not yet, and only to those who may make
        // projects there as well as move them.
        assert_eq!(
            flags,
            [
                MoveRight::IntoProjectsOrTop,
                MoveRight::IntoProjects,
                MoveRight::Nowhere,
                MoveRight::Nowhere
            ]
        );
        assert_eq!(
            detail(&org, &admin, "ACME").await?.may_move,
            MoveRight::IntoProjects
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_03_moving_to_the_top_level_needs_make_projects_as_well() -> TestResult {
    each_database("prj_03_top_level_needs_make", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let web = make(&org, &admin, None, "WEB").await?;
        let docs = make(&org, &admin, Some(web.id), "DOCS").await?;
        let mover = org
            .role(
                "Mover",
                &[Permission::SeeProjects, Permission::MoveProjects],
            )
            .await?;
        let maker = org.role("Maker", &[Permission::MakeProjects]).await?;
        let bob = org.person().await?;
        org.give_everywhere(&bob, mover.id).await?;
        assert_eq!(
            refused(preview(&org, &bob, docs.id, None).await),
            Some(ErrorCode::Forbidden)
        );
        assert_eq!(
            refused(relocate(&org, &bob, docs.id, None, "x").await),
            Some(ErrorCode::Forbidden)
        );
        org.give_everywhere(&bob, maker.id).await?;
        let shown = preview(&org, &bob, docs.id, None).await?;
        relocate(&org, &bob, docs.id, None, &shown.preview).await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_03_the_places_a_project_can_go_leave_out_where_it_is_and_where_the_person_may_not_move()
-> TestResult {
    each_database("prj_03_movable_places", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = make(&org, &admin, None, "ACME").await?;
        let other = make(&org, &admin, None, "OTHER").await?;
        let third = make(&org, &admin, None, "THIRD").await?;
        let web = make(&org, &admin, Some(acme.id), "WEB").await?;
        make(&org, &admin, Some(web.id), "DOCS").await?;
        let lead = org.role("Section lead", &LEAD).await?;
        let looker = org.role("Looker", &VIEW).await?;
        let ana = org.person().await?;
        org.give(&ana, lead.id, acme.id).await?;
        org.give(&ana, lead.id, other.id).await?;
        org.give(&ana, looker.id, third.id).await?;
        // Not itself, not what is inside it, not where it is, and for Ana not where she only looks.
        let places = |who: &Member| {
            let who = who.clone();
            let org = &org;
            async move {
                Ok::<_, bb_app::auth::AppError>(names(
                    &list_places(org, &who, None, Some(web.id)).await?,
                ))
            }
        };
        assert_eq!(places(&ana).await?, ["OTHER"]);
        assert_eq!(places(&admin).await?, ["OTHER", "THIRD"]);
        // A project the person does not see, or that is not there, filters nothing.
        let everything = names(&list_places(&org, &ana, None, None).await?);
        let hidden = org.person().await?;
        assert!(
            list_places(&org, &hidden, None, Some(web.id))
                .await?
                .items
                .is_empty()
        );
        let missing = names(&list_places(&org, &ana, None, Some(missing_id())).await?);
        assert_eq!(missing, everything);
        Ok(())
    })
    .await
}
