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
        if !self.allows(Permission::ChangeIssues)
            && self.allows(Permission::ChangeOwnIssues)
            && self.reported(reported_by)
        {
            Permission::ChangeOwnIssues
        } else {
            Permission::ChangeIssues
        }
    }

    /// Whether the viewer may change the fields of the issue that they may change: they see it and
    /// hold *Change issues*, or they reported it and hold *Change own issues*.
    #[must_use]
    pub fn may_change_issue(&self, reported_by: UserId) -> bool {
        self.sees_issue(reported_by) && self.allows(self.permission_to_change_issue(reported_by))
    }

    /// Whether the viewer may delete the issue into the trash: they see it and hold *Delete
    /// issues*.
    #[must_use]
    pub fn may_delete_issue(&self, reported_by: UserId) -> bool {
        self.sees_issue(reported_by) && self.allows(Permission::DeleteIssues)
    }
}
