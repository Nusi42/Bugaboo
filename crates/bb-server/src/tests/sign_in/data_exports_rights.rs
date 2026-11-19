//! The file an administrator made of somebody else's data stays theirs to download only while they
//! hold *Download someone's data* (PRIV-03, ADR-0044, decision 4), on SQLite and PostgreSQL.

use bb_app::access::Permission;
use bb_app::auth::AppError;
use bb_app::privacy::export::{AskSomeonesData, DownloadExportQuery, ExportLinkQuery};
use bb_app::roles::ChangeRole;
use bb_core::ErrorCode;
use bb_core::id::{DataExportId, RoleId};
use bb_testing::{Failure, TestResult, each_database};

use super::files_http::run_jobs;
use super::policy_rig::{Member, Org};
use super::rig::{Rig, refused};

/// The signed link of the download of an export, as `who` asks for it.
async fn link(rig: &Rig, who: &Member, export: DataExportId) -> Result<String, AppError> {
    rig.api()
        .export_link(
            Rig::info(),
            ExportLinkQuery {
                session_token: who.session.clone(),
                export: Some(export),
            },
        )
        .await
        .map(|link| link.into_value().token)
}

/// Whether the file opens for `who` through `link`.
async fn opens(rig: &Rig, who: &Member, link: &str) -> Result<(), AppError> {
    rig.api()
        .download_export(
            Rig::info(),
            DownloadExportQuery {
                session_token: who.session.clone(),
                link: link.to_owned(),
            },
        )
        .await
        .map(|_| ())
}

/// Sets what the role allows.
async fn allow(rig: &Rig, admin: &Member, role: RoleId, permissions: &[Permission]) -> TestResult {
    let version = rig.role_version(role).await?;
    rig.api()
        .change_role(
            Rig::info(),
            ChangeRole {
                session_token: admin.session.clone(),
                role,
                version,
                name: None,
                permissions: Some(permissions.to_vec()),
                two_step: None,
            },
        )
        .await?;
    Ok(())
}

#[tokio::test]
async fn priv_03_an_administrator_who_lost_download_someones_data_cannot_download_it() -> TestResult
{
    each_database("priv_03_lost_download_someones_data", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let exporter = org
            .role("Exporter", &[Permission::DownloadSomeonesData])
            .await?;
        let (bob, carol) = (org.person().await?, org.person().await?);
        org.give_everywhere(&bob, exporter.id).await?;
        let made = rig
            .api()
            .ask_for_someones_data(
                Rig::info(),
                AskSomeonesData {
                    session_token: bob.session.clone(),
                    person: Some(carol.id),
                    reason: Some("Carol asked by letter".to_owned()),
                },
            )
            .await?
            .into_value();
        run_jobs(&rig).await?;
        let held = link(&rig, &bob, made.export.id).await?;
        opens(&rig, &bob, &held).await?;
        // The role loses the permission: neither a new link nor the one Bob holds opens the file.
        allow(&rig, &org.admin, exporter.id, &[]).await?;
        let not_found = Some(ErrorCode::NotFound);
        assert_eq!(refused(link(&rig, &bob, made.export.id).await), not_found);
        assert_eq!(refused(opens(&rig, &bob, &held).await), not_found);
        Ok::<(), Failure>(())
    })
    .await
}
