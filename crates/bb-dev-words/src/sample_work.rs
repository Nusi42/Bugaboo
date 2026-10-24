//! What TEST-SAMPLE-DATA holds the working together of the sample against (M4 WP-10): the workflows
//! with the one a project has of its own, the files with their previews, the notifications, and
//! what Contributors, Guests and Viewers did. It is the part `issues.work` of the dump, judged by
//! [`WorkPlan::judge`] as part of [`IssuePlan::problems`](crate::sample::IssuePlan::problems).

use serde_json::Value;

use crate::sample::WorkPlan;
use crate::sample_issues::{Judge, scaled};

/// The workflows a new organization starts with (*Simple*, *Bug*, *Development*) and the one the
/// sample adds.
const WORKFLOWS_AT_LEAST: usize = 4;

/// What the working together of a sample asked for `asked` issues comes to: the counts of the plan
/// scaled like the issues. The program that makes the sample and the judge of its dump both read
/// them from here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkCounts {
    /// Files on issues and comments.
    pub files: u64,
    /// Issues Contributors report, all of them together.
    pub contributor_reports: u64,
    /// Issues Guests report, all of them together.
    pub guest_reports: u64,
    /// Status changes of Team members.
    pub team_status_changes: u64,
    /// Changes of Contributors to the issues they reported.
    pub contributor_changes: u64,
    /// Issues Viewers follow, all of them together.
    pub viewer_follows: u64,
}

impl WorkPlan {
    /// The counts for a sample of `asked` issues out of `full`; each is at least one, so even a
    /// small sample has every kind of work.
    #[must_use]
    pub fn counts(&self, asked: u32, full: u32) -> WorkCounts {
        let scale = |count: u32| scaled(count, full, asked).max(1);
        WorkCounts {
            files: scale(self.files.total),
            contributor_reports: scale(self.reports.contributors),
            guest_reports: scale(self.reports.guests),
            team_status_changes: scale(self.team_status_changes),
            contributor_changes: scale(self.contributor_changes),
            viewer_follows: scale(self.viewer_follows),
        }
    }

    /// Judges the part `work` of the dump for a sample of `asked` issues out of `full`.
    pub(crate) fn judge(&self, judge: &mut Judge<'_>, work: &Value, asked: u32, full: u32) {
        let number = |value: &Value| value.as_u64().unwrap_or(0);
        self.judge_workflows(judge, work);

        let files = &work["files"];
        let total = number(&files["total"]);
        let counts = self.counts(asked, full);
        let planned = counts.files;
        judge.near("the number of files", total, planned);
        judge.near(
            "the number of pictures",
            number(&files["pictures"]),
            planned * u64::from(self.files.pictures_percent) / 100,
        );
        judge.present("PDF files", number(&files["pdfs"]));
        judge.present("files of comments", number(&files["on_comments"]));
        judge.exactly(
            "files without a preview",
            number(&files["without_preview"]),
            0,
        );
        judge.exactly(
            "files with a preview",
            number(&files["previews_ready"]),
            total,
        );

        let notifications = &work["notifications"];
        let made = number(&notifications["total"]);
        judge.present("notifications", made);
        judge.present(
            "assignment notifications",
            number(&notifications["by_kind"]["assigned"]),
        );
        judge.present(
            "notifications of followed issues",
            number(&notifications["by_kind"]["followed-activity"]),
        );
        judge.present(
            "notifications for Viewers",
            number(&notifications["to"]["viewer"]),
        );
        judge.exactly(
            "notifications of Guests about issues they did not report",
            number(&notifications["to_guests_about_others"]),
            0,
        );
        let read = number(&notifications["read"]);
        judge.present("notifications that were read", read);
        if read > made {
            judge.problem("more notifications were read than there are");
        }

        for (role, count) in [
            ("contributor", counts.contributor_reports),
            ("guest", counts.guest_reports),
        ] {
            judge.near(
                &format!("the issues reported by {role}s"),
                number(&work["reports"][role]),
                count,
            );
        }
        judge.near(
            "the status changes of Team members",
            number(&work["status_changes"]["team_member"]),
            counts.team_status_changes,
        );
        judge.near(
            "the changes of Contributors to their own issues",
            number(&work["own_changes"]["contributor"]),
            counts.contributor_changes,
        );
        judge.present(
            "status changes that asked for who does the work",
            number(&work["asked_for_who"]),
        );
        judge.near(
            "the issues Viewers follow",
            number(&work["follows"]["viewer"]),
            counts.viewer_follows,
        );
    }

    /// The starter workflows, and the one workflow of a project with its rule *Ask for*.
    fn judge_workflows(&self, judge: &mut Judge<'_>, work: &Value) {
        let workflows = work["workflows"].as_array().map_or(&[][..], Vec::as_slice);
        if workflows.len() < WORKFLOWS_AT_LEAST {
            judge.problem(&format!(
                "the sample has {} workflows, the plan has at least {WORKFLOWS_AT_LEAST} (the three starters and the one of {})",
                workflows.len(),
                self.own_workflow.project
            ));
        }
        let own = workflows
            .iter()
            .find(|workflow| workflow["name"] == self.own_workflow.name.as_str());
        match own {
            None => judge.problem(&format!(
                "the workflow {} is missing",
                self.own_workflow.name
            )),
            Some(own) => judge.present(
                &format!("status change with *Ask for* in {}", self.own_workflow.name),
                own["asks"].as_u64().unwrap_or(0),
            ),
        }
        let projects = work["own_workflow_projects"]
            .as_array()
            .map_or(&[][..], Vec::as_slice);
        if !projects
            .iter()
            .any(|project| project == self.own_workflow.project.as_str())
        {
            judge.problem(&format!(
                "the project {} does not use a workflow of its own",
                self.own_workflow.project
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::sample::Layout;
    use crate::sample_issues::tests::made;

    fn ok() -> (crate::sample::IssuePlan, u32, Value) {
        let plan = Layout::bundled().unwrap().issues;
        let asked = plan.small_total;
        let dump = made(&plan, asked);
        (plan, asked, dump)
    }

    #[test]
    fn the_work_of_the_plan_is_accepted() {
        let (plan, asked, dump) = ok();
        assert_eq!(plan.problems(asked, "run", &dump), Vec::<String>::new());
    }

    #[test]
    fn what_the_work_of_the_plan_has_has_to_be_there() {
        let (plan, asked, ok) = ok();
        for (path, bad, word) in [
            ("files.without_preview", json!(3), "without a preview"),
            ("files.pdfs", json!(0), "PDF"),
            ("files.on_comments", json!(0), "comments"),
            ("files.total", json!(1), "number of files"),
            ("notifications.read", json!(0), "were read"),
            (
                "notifications.to_guests_about_others",
                json!(2),
                "did not report",
            ),
            ("notifications.by_kind.assigned", json!(0), "assignment"),
            ("notifications.to.viewer", json!(0), "Viewers"),
            ("reports.guest", json!(0), "guests"),
            ("reports.contributor", json!(1), "contributors"),
            ("status_changes.team_member", json!(1), "Team members"),
            ("own_changes.contributor", json!(1), "Contributors"),
            ("asked_for_who", json!(0), "asked for who"),
            ("follows.viewer", json!(0), "follow"),
            ("workflows", json!([]), "workflows"),
            ("own_workflow_projects", json!([]), "of its own"),
        ] {
            let mut dump = ok.clone();
            let mut at = &mut dump["issues"]["work"];
            for part in path.split('.') {
                at = &mut at[part];
            }
            *at = bad;
            let problems = plan.problems(asked, "run", &dump);
            assert!(
                problems.iter().any(|p| p.contains(word)),
                "{path}: {problems:?}"
            );
        }
    }

    #[test]
    fn the_workflow_of_the_project_needs_a_rule_that_asks_for() {
        let (plan, asked, mut dump) = ok();
        dump["issues"]["work"]["workflows"][3]["asks"] = json!(0);
        let problems = plan.problems(asked, "run", &dump);
        assert!(
            problems.iter().any(|p| p.contains("Ask for")),
            "{problems:?}"
        );
    }
}
