//! The large dataset of PERF-01 (architecture §11, *Large dataset*): the numbers
//! `cargo xtask dataset large` makes the sample's kinds of things at, written in
//! `tools/dev/sample.json` under `large`. The shape (which kinds of things exist) is the sample's;
//! only the scale and the shares here are new.

use serde::Deserialize;

use crate::sample::LayoutError;

/// What the large dataset holds, and in what shares.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LargePlan {
    /// How many issues `cargo xtask dataset large` makes when no number is given.
    pub issues: u32,
    /// How many issues the runs of `cargo xtask dataset large --check` make.
    pub check_issues: u32,
    /// The folder the datasets are kept in, relative to the work folder (`/` as separator): one
    /// folder for each database.
    pub out: String,
    /// The folder of the runs of `--check`.
    pub check_out: String,
    /// Where the numbers of the benchmarks are kept.
    pub bench_out: String,
    /// Where the log of the nightly run is kept.
    pub nightly_out: String,
    /// The name of the PostgreSQL database that holds the large dataset.
    pub database: String,
    /// How many people there are in all: the sample's people and the rest.
    pub people: u32,
    /// How many projects there are in all: the sample's and the rest.
    pub projects: u32,
    /// How many sub-projects each project of the rest has at most (a project with its
    /// sub-projects is one family; the sample's four families keep theirs).
    pub sub_projects_each: u32,
    /// How many groups there are in all: the sample's and the rest.
    pub groups: u32,
    /// How many of the projects of the rest have a workflow of their own.
    pub own_workflow_projects: u32,
    /// How many months back the first issue was made.
    pub months: u32,
    /// How many comments there are on 100 issues.
    pub comments_per_100: u32,
    /// How many of 100 projects write in German (the rest in English); full-text search needs both.
    pub german_percent: u32,
    /// How many of 100 of the oldest issues are done, and how many of the newest; in between it
    /// goes in a line. An issue that is not done stands at a status on the way to done.
    pub done_percent_oldest: u32,
    /// See [`Self::done_percent_oldest`].
    pub done_percent_newest: u32,
    /// How many relations there are on 100 issues.
    pub relations_per_100: u32,
    /// How many of 100 issues are sub-issues of another one.
    pub sub_issue_percent: u32,
    /// How many of 100 issues carry tags.
    pub tag_percent: u32,
    /// How many of 100 issues carry a label.
    pub label_percent: u32,
    /// How many of 100 issues carry a file.
    pub file_percent: u32,
    /// How many of 100 issues are assigned to somebody.
    pub assigned_percent: u32,
    /// How many people at most follow an issue besides the one who reported it.
    pub followers_at_most: u32,
    /// How many of 100 events, the newest ones, the notifications are made for. Older events
    /// are marked as read by the notification reader without a notification, as a server that has
    /// been running for a long time has cleaned them up.
    pub notified_percent: u32,
    /// The same for a dataset of no more than `check_issues` issues (the runs of `--check`): the
    /// rules of what Guests and Contributors are told need notifications to look at, and in a
    /// small dataset the newest 2 % hold none of theirs.
    pub check_notified_percent: u32,
    /// How many issues one transaction writes.
    pub batch: u32,
    /// How many of 100 issues are in the biggest tenth of the projects.
    pub big_projects_share: u32,
}

/// The folder every folder of the large dataset is inside of, relative to the work folder: the
/// datasets are thrown away and made again there, so nothing else may be named.
const WORK_FOLDER: &str = ".work";

/// Whether `folder` is a folder inside `.work/`: written with `/`, not empty, not absolute and
/// without `..` (`xtask` removes `<out>/<database>` before it makes a dataset).
fn inside_work_folder(folder: &str) -> bool {
    folder
        .strip_prefix(WORK_FOLDER)
        .and_then(|rest| rest.strip_prefix('/'))
        .is_some_and(|rest| {
            !rest.is_empty()
                && !rest.contains(['\\', ':'])
                && rest.split('/').all(|part| !part.is_empty() && part != "..")
        })
}

impl LargePlan {
    /// What is wrong with the plan, in a sentence, if anything.
    pub(crate) fn check(&self) -> Result<(), LayoutError> {
        let folders = [
            ("out", &self.out),
            ("checkOut", &self.check_out),
            ("benchOut", &self.bench_out),
            ("nightlyOut", &self.nightly_out),
        ];
        if let Some((name, _)) = folders
            .iter()
            .find(|(_, folder)| !inside_work_folder(folder))
        {
            return Err(LayoutError::new(format!(
                "large.{name} must be a folder inside {WORK_FOLDER}/ (relative, with /, without ..)"
            )));
        }
        let percents = [
            ("germanPercent", self.german_percent),
            ("subIssuePercent", self.sub_issue_percent),
            ("tagPercent", self.tag_percent),
            ("labelPercent", self.label_percent),
            ("filePercent", self.file_percent),
            ("assignedPercent", self.assigned_percent),
            ("donePercentOldest", self.done_percent_oldest),
            ("donePercentNewest", self.done_percent_newest),
            ("notifiedPercent", self.notified_percent),
            ("checkNotifiedPercent", self.check_notified_percent),
            ("bigProjectsShare", self.big_projects_share),
        ];
        if let Some((name, _)) = percents.iter().find(|(_, value)| *value > 100) {
            return Err(LayoutError::new(format!(
                "large.{name} is a share of 100, not more"
            )));
        }
        let positive = [
            ("issues", self.issues),
            ("checkIssues", self.check_issues),
            ("people", self.people),
            ("projects", self.projects),
            ("months", self.months),
            ("batch", self.batch),
        ];
        if let Some((name, _)) = positive.iter().find(|(_, value)| *value == 0) {
            return Err(LayoutError::new(format!("large.{name} must be at least 1")));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::sample::Layout;

    #[test]
    fn the_bundled_large_plan_is_the_perf_scale() {
        let large = Layout::bundled().unwrap().large;
        assert_eq!(large.issues, 1_000_000);
        assert_eq!((large.people, large.projects), (500, 500));
    }

    #[test]
    fn a_share_over_a_hundred_or_a_count_of_zero_is_refused() {
        let text = include_str!("../../../tools/dev/sample.json");
        let broken = text.replace("\"germanPercent\": 30", "\"germanPercent\": 130");
        assert!(Layout::parse(&broken).is_err());
        let broken = text.replace("\"batch\": 2000", "\"batch\": 0");
        assert!(Layout::parse(&broken).is_err());
    }

    #[test]
    fn a_folder_of_the_large_dataset_must_be_inside_the_work_folder() {
        let text = include_str!("../../../tools/dev/sample.json");
        for name in ["out", "checkOut", "benchOut", "nightlyOut"] {
            let line = text
                .lines()
                .find(|line| {
                    line.trim_start()
                        .starts_with(&format!("\"{name}\": \".work/"))
                })
                .unwrap();
            for bad in [
                "",
                "target/x",
                "/x",
                "C:/x",
                ".work",
                ".work/",
                ".work/../x",
                ".work//x",
            ] {
                let broken = text.replace(line, &format!("    \"{name}\": \"{bad}\","));
                assert!(Layout::parse(&broken).is_err(), "{name} = {bad:?}");
            }
        }
    }
}
