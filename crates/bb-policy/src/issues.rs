//! Which issues a person sees and may change in a project (ISS-01, SEC-04, M3-Q6).
//!
//! The permissions *See issues*, *Change issues*, *Change own issues* and *Delete issues* count in
//! a project like every other; what is added here is the part of the rule that depends on the
//! issue: *See issues* shows every issue of the project, and without it a person still sees the
//! issues they reported (a Guest makes an issue and finds it again). The rule is written once,
//! here, and the use cases ask it (ARCH-AUTHZ-ONLY).

use bb_core::id::UserId;

use crate::catalogue::Permission;
use crate::evaluate::Evaluation;

/// The permissions that let a person change issues: *Change issues* for every issue they see, and
/// *Change own issues* for the ones they reported. The one list of the pair.
pub const CHANGES_ISSUES: [Permission; 2] = [Permission::ChangeIssues, Permission::ChangeOwnIssues];

impl Evaluation {
    /// Whether the viewer reported the issue, which `reported_by` names.
    fn reported(&self, reported_by: UserId) -> bool {
        self.person() == Some(reported_by)
    }

    /// Whether the viewer sees an issue of the project that `reported_by` reported: they see the
    /// project and hold *See issues*, or they reported the issue themselves. An issue they do not
    /// see answers like one that does not exist.
    #[must_use]
    pub fn sees_issue(&self, reported_by: UserId) -> bool {
        self.sees_project() && (self.allows(Permission::SeeIssues) || self.reported(reported_by))
    }

    /// The permission that lets the viewer change the issue: *Change own issues* for somebody who
    /// reported it and holds that one and not *Change issues*, otherwise *Change issues* (which
    /// somebody who holds neither lacks). The one place that tells own from all.
    #[must_use]
    pub fn permission_to_change_issue(&self, reported_by: UserId) -> Permission {
        let [all, own] = CHANGES_ISSUES;
        if !self.allows(all) && self.allows(own) && self.reported(reported_by) {
            own
        } else {
            all
        }
    }

    /// Whether the viewer may change the fields of the issue that they may change: they see it and
    /// hold *Change issues*, or they reported it and hold *Change own issues*.
    #[must_use]
    pub fn may_change_issue(&self, reported_by: UserId) -> bool {
        self.sees_issue(reported_by) && self.allows(self.permission_to_change_issue(reported_by))
    }

    /// Whether the viewer may change some issue of the project: they see it and hold *Change
    /// issues* or *Change own issues* ([`CHANGES_ISSUES`]), for when no issue is named yet (which
    /// tools of AI to offer).
    #[must_use]
    pub fn may_change_some_issue(&self) -> bool {
        self.sees_project() && CHANGES_ISSUES.iter().any(|each| self.allows(*each))
    }

    /// Whether the viewer may delete the issue into the trash: they see it and hold *Delete
    /// issues*.
    #[must_use]
    pub fn may_delete_issue(&self, reported_by: UserId) -> bool {
        self.sees_issue(reported_by) && self.allows(Permission::DeleteIssues)
    }
}
