//! Guessing access keys is limited per network, and a key that works is never held up by it
//! (SEC-14, AUTH-07): only a failed lookup counts, and only a failed lookup is answered with a wait.

use bb_app::auth::{AppError, RequestInfo};
use bb_app::roles::CheckAccessQuery;
use bb_auth::AccessKeySecret;
use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::network::NetworkPrefix;
use bb_testing::{Failure, TestResult, each_database};

use super::access_keys::key_of;
use super::policy_rig::Org;
use super::rig::{Rig, refused};

/// A request from a network, with whatever the caller holds: *Check access* for the administrator.
async fn probe_from(rig: &Rig, network: &str, credential: &str) -> Result<(), AppError> {
    let from = RequestInfo {
        network: NetworkPrefix::from_stored(network),
        ..RequestInfo::default()
    };
    let ada = rig
        .person_id("ada@example.test")
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    rig.api()
        .check_access(
            from,
            CheckAccessQuery {
                session_token: credential.to_owned(),
                person: ada,
                place: None,
            },
        )
        .await
        .map(|_| ())
}

fn a_guess() -> Result<String, Failure> {
    Ok(AccessKeySecret::generate()?.encode().as_str().to_owned())
}

const HERE: &str = "203.0.113.0";

#[tokio::test]
async fn sec_14_keys_that_work_are_never_turned_away_when_they_are_used_at_the_same_time()
-> TestResult {
    each_database("sec_14_parallel_working_keys", async |db| {
        // Two failed tries per network and minute: a limit that parallel use would pass at once if
        // every request counted as a try while it is looked at.
        let rig = Rig::start(db, 2).await?;
        let org = Org::start(&rig).await?;
        let key = key_of(&rig, &org.admin, KeyChoice::Everything).await?;
        let call = || probe_from(&rig, HERE, &key);
        let answers = tokio::join!(
            call(),
            call(),
            call(),
            call(),
            call(),
            call(),
            call(),
            call()
        );
        let answers = [
            answers.0, answers.1, answers.2, answers.3, answers.4, answers.5, answers.6, answers.7,
        ];
        for answer in answers {
            assert_eq!(refused(answer), None);
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_14_a_network_over_its_limit_is_made_to_wait_but_a_key_that_works_still_passes()
-> TestResult {
    each_database("sec_14_over_the_limit_a_working_key_passes", async |db| {
        let rig = Rig::start(db, 3).await?;
        let org = Org::start(&rig).await?;
        let key = key_of(&rig, &org.admin, KeyChoice::Everything).await?;
        // A neighbour on the same network guesses until the network is over its limit.
        for _ in 0..3 {
            assert_eq!(
                refused(probe_from(&rig, HERE, &a_guess()?).await),
                Some(ErrorCode::NotSignedIn)
            );
        }
        // Another guess is made to wait: that is the answer of the limit, and only for a guess.
        assert!(matches!(
            probe_from(&rig, HERE, &a_guess()?).await,
            Err(AppError::Limited { .. })
        ));
        // The key of somebody on that network works all the same, as often as it is used, and it
        // does not take anything from the failures of the network.
        for _ in 0..10 {
            assert_eq!(refused(probe_from(&rig, HERE, &key).await), None);
        }
        assert!(matches!(
            probe_from(&rig, HERE, &a_guess()?).await,
            Err(AppError::Limited { .. })
        ));
        // Another network is not affected by this one at all.
        assert_eq!(
            refused(probe_from(&rig, "198.51.100.0", &a_guess()?).await),
            Some(ErrorCode::NotSignedIn)
        );
        Ok(())
    })
    .await
}
