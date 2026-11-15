//! What TEST-SAMPLE-DATA holds the plan of the sample against (M8 WP-12, part 7): the part
//! `issues.work.timeline` of the dump, judged by [`TimelinePlan::judge`] as part of
//! [`WorkPlan::judge`](crate::sample::WorkPlan). The goal *Mobile app 2.0* has its issues with
//! their dates, estimates and dependencies, the person who plans has usual work days, a share and
//! no away time, and one issue carries her own estimate. Days are written as the number of days
//! before the due date of the goal, so the dump does not depend on the day it was made.

use bb_core::issue::{IssueIcon, STARTING_ISSUE_TYPES};
use bb_core::message::Message;
use serde_json::Value;

use crate::sample::TimelinePlan;
use crate::sample_issues::Judge;

impl TimelinePlan {
    /// How many issues the plan makes besides the issues of the plan of the sample: the goal and
    /// its issues.
    pub(crate) fn made_issues(&self) -> u64 {
        1 + self.issues.len() as u64
    }

    /// How many issues of each type the plan makes: the goal, and its issues, which are tasks.
    pub(crate) fn made_types(&self) -> [(String, u64); 2] {
        let name = |icon| {
            STARTING_ISSUE_TYPES
                .iter()
                .find(|starting| starting.icon == icon)
                .map(|starting| Message::from(starting.name).render())
                .unwrap_or_default()
        };
        [
            (name(IssueIcon::Flag), 1),
            (name(IssueIcon::SquareCheck), self.issues.len() as u64),
        ]
    }

    /// Judges the part `timeline` of the dump.
    pub(crate) fn judge(&self, judge: &mut Judge<'_>, timeline: &Value) {
        let goal = &timeline["goal"];
        if goal["title"] != self.goal.title.as_str() {
            judge.problem(&format!(
                "the goal is {}, not {}",
                goal["title"], self.goal.title
            ));
        }
        judge.exactly(
            "dated sub-issues of the goal",
            goal["sub_issues"].as_u64().unwrap_or(0),
            self.issues.len() as u64,
        );
        let found = timeline["issues"].as_array().map_or(&[][..], Vec::as_slice);
        for planned in &self.issues {
            let Some(made) = found
                .iter()
                .find(|made| made["title"] == planned.title.as_str())
            else {
                judge.problem(&format!("the issue {} is missing", planned.title));
                continue;
            };
            if made["person"] != planned.person.as_str() {
                judge.problem(&format!(
                    "{} is assigned to {}",
                    planned.title, made["person"]
                ));
            }
            for (what, found, day) in [
                ("starts", &made["starts_before_due"], planned.start_day),
                ("ends", &made["ends_before_due"], planned.due_day),
            ] {
                let planned_days = self.goal.due_day - day;
                if found.as_i64() != Some(planned_days) {
                    judge.problem(&format!(
                        "{} {what} {found} days before the due date of the goal, the plan says {planned_days}",
                        planned.title
                    ));
                }
            }
            judge.exactly(
                &format!("minutes of estimate of {}", planned.title),
                made["estimate"].as_u64().unwrap_or(0),
                u64::from(planned.estimate),
            );
        }
        let blocks = timeline["blocks"].as_array().map_or(&[][..], Vec::as_slice);
        judge.exactly(
            "dependencies of blocks",
            blocks.len() as u64,
            self.blocks.len() as u64,
        );
        for [first, waiting] in &self.blocks {
            let there = blocks
                .iter()
                .any(|each| each[0] == first.as_str() && each[1] == waiting.as_str());
            if !there {
                judge.problem(&format!("{first} does not block {waiting}"));
            }
        }
        self.judge_times(judge, timeline);
    }

    /// The work times of the person who plans, and the one own estimate.
    fn judge_times(&self, judge: &mut Judge<'_>, timeline: &Value) {
        let number = |value: &Value| value.as_u64().unwrap_or(0);
        if timeline["holidays"] != self.holidays.as_str() {
            judge.problem(&format!(
                "the public holidays are {}, the plan has {}",
                timeline["holidays"], self.holidays
            ));
        }
        let person = &timeline["person"];
        let usual: Vec<(bool, u64)> = person["usual_days"]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .map(|day| (day[0] == true, number(&day[1])))
            .collect();
        let planned: Vec<(bool, u64)> = self
            .usual_days
            .iter()
            .map(|works| (*works, u64::from(self.usual_minutes)))
            .collect();
        if usual != planned {
            judge.problem(&format!(
                "the usual work days of {} are {usual:?}, the plan has {planned:?}",
                self.person
            ));
        }
        judge.exactly(
            "percent of share in the project",
            number(&person["share_percent"]),
            u64::from(self.share_percent),
        );
        judge.exactly("away times of the person", number(&person["away_times"]), 0);
        judge.exactly(
            "people the issues of the goal are assigned to",
            number(&timeline["people_on_the_goal"]),
            1 + self.colleagues.len() as u64,
        );
        let own = timeline["own_estimates"]
            .as_array()
            .map_or(&[][..], Vec::as_slice);
        judge.exactly("own estimates", own.len() as u64, 1);
        for estimate in own {
            if estimate["issue"] != self.own_estimate.issue.as_str()
                || estimate["set_by_the_person"] != true
            {
                judge.problem(&format!(
                    "the own estimate is not by the person on {}",
                    estimate["issue"]
                ));
            }
        }
    }
}
