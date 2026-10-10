//! Following an issue (ISS-07): who follows, and the one rule for who starts following by what
//! they do. Delivery (e-mail, the bell) comes in M4; in M3 followers are stored and shown.

use jiff::Timestamp;

use crate::id::{IssueId, UserId};
use crate::personal::Entity;

words! {
    /// Why a person follows an issue.
    pub enum FollowReason {
        /// They reported the issue.
        Reported => "reported",
        /// The issue was assigned to them.
        Assigned => "assigned",
        /// They commented on it.
        Commented => "commented",
        /// They chose to follow it (*Follow*).
        Chose => "chose",
    }
}

words_in_api!(
    FollowReason,
    "Why a person follows an issue: they reported it, were assigned to it, commented on it or chose to."
);

/// A person who follows an issue.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Follower {
    /// The issue.
    #[not_personal]
    pub issue: IssueId,
    /// Who follows it.
    #[personal(category = "activity", basis = "legitimate_interests")]
    pub person: UserId,
    /// Why.
    #[not_personal]
    pub reason: FollowReason,
    /// Since when.
    #[not_personal]
    pub since: Timestamp,
}

/// Who starts following because of what just happened, each person once, with the first reason
/// that applies. The one rule (ISS-07): people who report an issue, are assigned to it or comment
/// on it follow it. The use cases that report, assign and comment ask this and nothing else.
#[must_use]
pub fn who_starts_following(
    reporter: Option<UserId>,
    assigned: &[UserId],
    commenter: Option<UserId>,
) -> Vec<(UserId, FollowReason)> {
    let mut found: Vec<(UserId, FollowReason)> = Vec::new();
    let candidates = reporter
        .into_iter()
        .map(|person| (person, FollowReason::Reported))
        .chain(
            assigned
                .iter()
                .map(|person| (*person, FollowReason::Assigned)),
        )
        .chain(
            commenter
                .into_iter()
                .map(|person| (person, FollowReason::Commented)),
        );
    for (person, reason) in candidates {
        if !found.iter().any(|(known, _)| *known == person) {
            found.push((person, reason));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::SequentialIds;

    #[test]
    fn iss_07_reporting_assigning_and_commenting_each_start_following_once_per_person() {
        let mut ids = SequentialIds::default();
        let (ana, ben, cleo) = (
            UserId::generate(&mut ids),
            UserId::generate(&mut ids),
            UserId::generate(&mut ids),
        );
        assert_eq!(
            who_starts_following(Some(ana), &[ana, ben], Some(cleo)),
            [
                (ana, FollowReason::Reported),
                (ben, FollowReason::Assigned),
                (cleo, FollowReason::Commented)
            ]
        );
        assert!(who_starts_following(None, &[], None).is_empty());
        assert_eq!(
            who_starts_following(None, &[], Some(ben)),
            [(ben, FollowReason::Commented)]
        );
    }
}
