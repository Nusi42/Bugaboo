//! The example requests of the routes of *Switch off* and *Switch on* (PRIV-04).
//!
//! Each example names a person of its own, made for it by the administrator of the matrix, so every
//! actor who is let in switches off (or on) somebody who is active (or switched off) at that moment.

use bb_app::people::SwitchOff;
use bb_testing::Failure;

use super::examples::Request;
use super::routes::Fixture;
use crate::sign_in_tests::rig::Rig;

/// A person of their own for one example, made by the administrator of the matrix.
async fn a_new_person(fixture: &Fixture<'_>) -> Result<bb_core::id::UserId, Failure> {
    let email = format!("switched-{}@example.test", fixture.next_number());
    Ok(fixture
        .rig()
        .person(fixture.admin_session(), &email)
        .await
        .map_err(|error| error.to_string())?
        .person
        .id)
}

pub(super) async fn people_state_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
    Ok(Some(match operation {
        "switchPersonOff" => (
            "POST",
            format!("/api/v1/people/{}/switch-off", a_new_person(fixture).await?),
            None,
        ),
        "switchPersonOn" => {
            let person = a_new_person(fixture).await?;
            fixture
                .rig()
                .api()
                .switch_off(
                    Rig::info(),
                    SwitchOff {
                        session_token: fixture.admin_session().to_owned(),
                        person,
                    },
                )
                .await
                .map_err(|error| error.to_string())?;
            ("POST", format!("/api/v1/people/{person}/switch-on"), None)
        }
        _ => return Ok(None),
    }))
}
