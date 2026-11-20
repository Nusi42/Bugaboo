//! The people *Download someone's data* offers to choose from (PRIV-03, ADR-0044, decision 4): a
//! holder of that permission picks people without *Manage people*, and sees them as *Who sees
//! people* lets them, so the picker names nobody the screens would not name.

use bb_core::ErrorCode;
use bb_core::access::PeopleLevel;
use bb_core::people_kinds::PeopleKind;

use super::data_exports::{may_download_someones_data, see};
use super::notifications::Crew;
use super::scenarios::text;
use super::world::Outcome;
use crate::auth::AppError;
use crate::people::{ChoicePage, PeopleQuery, people};
use crate::privacy::export::{ExportPeopleQuery, export_people};

/// The people Bob may pick, by name.
async fn picked_by_bob(crew: &Crew) -> Result<ChoicePage, AppError> {
    export_people(
        &crew.world.ctx(),
        ExportPeopleQuery {
            session_token: crew.bob.session_token.clone(),
            search: None,
            cursor: None,
            limit: 50,
        },
    )
    .await
    .map(crate::access::Authorized::into_value)
}

/// The names of a page of choices.
fn names(page: &ChoicePage) -> Vec<&str> {
    page.items
        .iter()
        .map(|choice| choice.name.as_str())
        .collect()
}

#[tokio::test]
async fn priv_03_a_holder_of_download_someones_data_picks_people_without_manage_people() -> Outcome
{
    let crew = Crew::new().await?;
    // Without the permission, nobody is offered.
    assert!(matches!(
        picked_by_bob(&crew).await,
        Err(AppError::Refused(ErrorCode::Forbidden))
    ));
    may_download_someones_data(&crew, true).await?;
    // The list of *Manage people* stays closed to Bob, but the picker offers Carol by name.
    let managed = people(
        &crew.world.ctx(),
        PeopleQuery {
            session_token: crew.bob.session_token.clone(),
            search: None,
            cursor: None,
            limit: 50,
            groups: false,
            roles: false,
        },
    )
    .await;
    assert!(matches!(
        managed,
        Err(AppError::Refused(ErrorCode::Forbidden))
    ));
    see(&crew, PeopleKind::ProjectMembers, PeopleLevel::Names).await?;
    let page = picked_by_bob(&crew).await.map_err(text)?;
    assert!(
        names(&page).contains(&crew.carol.person.display_name.as_str()),
        "{:?}",
        names(&page)
    );
    // Where *Who sees people* hides the names of project members from Bob, the picker names nobody.
    see(&crew, PeopleKind::ProjectMembers, PeopleLevel::Hidden).await?;
    let page = picked_by_bob(&crew).await.map_err(text)?;
    assert!(
        !names(&page).contains(&crew.carol.person.display_name.as_str()),
        "{:?}",
        names(&page)
    );
    Ok(())
}
