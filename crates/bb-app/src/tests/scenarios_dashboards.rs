//! The scenarios of the dashboard use cases (ADR-0007, M1-F18, CFG-03): the preparation, the call,
//! and how many events the call added. `events.rs` names each use case once and runs them.

use bb_core::dashboard::{BoxKind, BoxOptions, BoxPlace, BoxSpec};
use bb_core::saved_search::ShareTarget;

use super::scenarios::{Added, a_project, measured, text, token};
use super::world::World;
use crate::dashboards::{
    BoxChange, ChangeDashboard, CopyDashboard, CreateDashboard, DashboardView, DeleteDashboard,
    RestoreDashboard, ShareDashboard, StopSharingDashboard, change_dashboard, copy_dashboard,
    create_dashboard, delete_dashboard, restore_dashboard, share_dashboard, stop_sharing_dashboard,
};

/// *My timer*, which reads no search, at the top left: a box any world can keep.
fn a_timer() -> BoxSpec {
    BoxSpec {
        kind: BoxKind::MyTimer,
        place: BoxPlace {
            column: 0,
            row: 0,
            width: 3,
            height: 2,
        },
        title: "Timer".to_owned(),
        source: None,
        options: BoxOptions::default(),
    }
}

/// A dashboard of the administrator with one box.
async fn adas_dashboard(world: &World) -> Result<DashboardView, String> {
    world.clock.advance(1);
    Ok(create_dashboard(
        &world.ctx(),
        CreateDashboard {
            session_token: token(world),
            name: "Team".to_owned(),
            boxes: vec![a_timer()],
        },
    )
    .await
    .map_err(text)?
    .into_value())
}

/// Shares the dashboard of the administrator with a new project.
async fn shared_with_a_project(world: &World, made: &DashboardView) -> Result<ShareTarget, String> {
    let target = ShareTarget::Project(a_project(world, "ACME").await?);
    share_dashboard(
        &world.ctx(),
        ShareDashboard {
            session_token: token(world),
            dashboard: made.id,
            target,
        },
    )
    .await
    .map_err(text)?;
    Ok(target)
}

pub(crate) async fn create_dashboard_scenario(world: &World) -> Added {
    Box::pin(measured(&world.store, adas_dashboard(world))).await
}

pub(crate) async fn change_dashboard_scenario(world: &World) -> Added {
    let made = adas_dashboard(world).await?;
    let wider = BoxSpec {
        place: BoxPlace {
            width: 6,
            ..a_timer().place
        },
        ..a_timer()
    };
    Box::pin(measured(
        &world.store,
        change_dashboard(
            &world.ctx(),
            ChangeDashboard {
                session_token: token(world),
                dashboard: made.id,
                version: made.version,
                name: Some("Our team".to_owned()),
                boxes: Some(vec![BoxChange {
                    id: made.boxes.first().map(|each| each.id),
                    spec: wider,
                }]),
            },
        ),
    ))
    .await
}

pub(crate) async fn share_dashboard_scenario(world: &World) -> Added {
    let made = adas_dashboard(world).await?;
    let target = ShareTarget::Project(a_project(world, "ACME").await?);
    Box::pin(measured(
        &world.store,
        share_dashboard(
            &world.ctx(),
            ShareDashboard {
                session_token: token(world),
                dashboard: made.id,
                target,
            },
        ),
    ))
    .await
}

pub(crate) async fn stop_sharing_dashboard_scenario(world: &World) -> Added {
    let made = adas_dashboard(world).await?;
    let target = shared_with_a_project(world, &made).await?;
    Box::pin(measured(
        &world.store,
        stop_sharing_dashboard(
            &world.ctx(),
            StopSharingDashboard {
                session_token: token(world),
                dashboard: made.id,
                target,
            },
        ),
    ))
    .await
}

pub(crate) async fn delete_dashboard_scenario(world: &World) -> Added {
    let made = adas_dashboard(world).await?;
    Box::pin(measured(
        &world.store,
        delete_dashboard(
            &world.ctx(),
            DeleteDashboard {
                session_token: token(world),
                dashboard: made.id,
            },
        ),
    ))
    .await
}

pub(crate) async fn restore_dashboard_scenario(world: &World) -> Added {
    let made = adas_dashboard(world).await?;
    delete_dashboard(
        &world.ctx(),
        DeleteDashboard {
            session_token: token(world),
            dashboard: made.id,
        },
    )
    .await
    .map_err(text)?;
    Box::pin(measured(
        &world.store,
        restore_dashboard(
            &world.ctx(),
            RestoreDashboard {
                session_token: token(world),
                dashboard: made.id,
            },
        ),
    ))
    .await
}

pub(crate) async fn copy_dashboard_scenario(world: &World) -> Added {
    let made = adas_dashboard(world).await?;
    Box::pin(measured(
        &world.store,
        copy_dashboard(
            &world.ctx(),
            CopyDashboard {
                session_token: token(world),
                dashboard: made.id,
                name: None,
            },
        ),
    ))
    .await
}
