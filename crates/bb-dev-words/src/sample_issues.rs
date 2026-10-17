//! What TEST-SAMPLE-DATA holds the issues of a made sample against: the plan of
//! `tools/dev/sample.json`, scaled to the number of issues the sample was asked to make. The
//! dump of the sample (`dump.json`, its part `issues`) says what was made; [`IssuePlan::problems`]
//! says in plain words what does not match. `xtask` (`cargo xtask dataset sample --check`) and the
//! tests of `bb-sample` both ask it, so the plan is judged in one place.

use bb_core::relation::STARTING_RELATION_KINDS;
use serde_json::Value;

use crate::sample::{IssuePlan, ProjectIssues, TypeShare};

/// `count` of the full sample scaled to a sample of `asked` issues out of `full`, rounded.
pub(crate) fn scaled(count: u32, full: u32, asked: u32) -> u64 {
    if full == 0 {
        return 0;
    }
    (u64::from(count) * u64::from(asked) + u64::from(full) / 2) / u64::from(full)
}

/// What is wrong, collected in plain words.
pub(crate) struct Judge<'a> {
    label: &'a str,
    percent: u32,
    problems: Vec<String>,
}

impl Judge<'_> {
    /// Something that is wrong in a way of its own.
    pub(crate) fn problem(&mut self, what: &str) {
        self.problems.push(format!(
            "{}: {what}. The sample has the work of architecture §11.",
            self.label
        ));
    }

    /// A made count has to be near the planned one: `percent` of it apart at most, and one.
    pub(crate) fn near(&mut self, what: &str, found: u64, planned: u64) {
        let allowed = (planned * u64::from(self.percent)).div_ceil(100).max(1);
        if found.abs_diff(planned) > allowed {
            self.problems.push(format!(
                "{}: {what} is {found}, the plan says {planned} (at most {allowed} apart). The sample has the issues of architecture §11.",
                self.label
            ));
        }
    }

    /// Something the plan has must be there at all.
    pub(crate) fn present(&mut self, what: &str, found: u64) {
        if found == 0 {
            self.problems.push(format!(
                "{}: the sample has no {what}. The plan of architecture §11 has them.",
                self.label
            ));
        }
    }

    /// Something the plan has exactly so many of.
    pub(crate) fn exactly(&mut self, what: &str, found: u64, planned: u64) {
        if found != planned {
            self.problems.push(format!(
                "{}: the sample has {found} {what}, the plan has {planned}.",
                self.label
            ));
        }
    }
}

impl IssuePlan {
    /// The shares of the types of a project: its own, or those of the whole sample.
    fn types_of<'a>(&'a self, project: &'a ProjectIssues) -> &'a [TypeShare] {
        project.types.as_deref().unwrap_or(&self.types)
    }

    /// What does not match the plan in the `issues` part of a dump, in plain words; empty when
    /// the sample has what the plan says, within `tolerancePercent`. `asked` is how many issues
    /// the sample was asked to make; `label` says which run the dump is of.
    #[must_use]
    pub fn problems(&self, asked: u32, label: &str, dump: &Value) -> Vec<String> {
        let mut judge = Judge {
            label,
            percent: self.tolerance_percent,
            problems: Vec::new(),
        };
        let issues = &dump["issues"];
        let number = |value: &Value| value.as_u64().unwrap_or(0);
        judge.exactly(
            "issues asked for",
            number(&issues["planned"]),
            u64::from(asked),
        );
        let full: u32 = self.projects.iter().map(|project| project.issues).sum();
        self.judge_projects(&mut judge, issues, asked, full);

        let relations = issues["relations"].as_object();
        let made: u64 = relations.map_or(0, |kinds| kinds.values().map(number).sum());
        judge.near(
            "the number of relations",
            made,
            scaled(self.relations, full, asked),
        );
        judge.present(
            "relations of every kind",
            u64::from(relations.map_or(0, serde_json::Map::len) >= STARTING_RELATION_KINDS.len()),
        );
        judge.present(
            "chain of *blocks* deeper than four steps",
            u64::from(number(&issues["longest_blocks_chain"]) > 4),
        );
        judge.exactly(
            "circles of *blocks* (the one deliberate circle)",
            number(&issues["blocks_circles"]),
            1,
        );
        judge.present("sub-issues under goals", number(&issues["sub_issues"]));
        judge.present("tags on issues", number(&issues["tags_on_issues"]));
        judge.present("labels on issues", number(&issues["labels_on_issues"]));
        for field in &self.fields {
            judge.present(
                &format!("values of the own field {}", field.name),
                number(&issues["own_field_values"][field.name.as_str()]),
            );
        }
        judge.near(
            "the number of comments",
            number(&issues["comments"]),
            scaled(self.comments, full, asked).max(1),
        );
        judge.present("edited comments", number(&issues["edited_comments"]));
        judge.exactly("edit locks held", number(&issues["edit_locks"]), 1);
        judge.exactly("drafts waiting", number(&issues["drafts"]), 1);
        self.work.judge(&mut judge, &issues["work"], asked, full);
        judge.problems
    }

    /// The issues in each project and place, and of each type.
    fn judge_projects(&self, judge: &mut Judge<'_>, issues: &Value, asked: u32, full: u32) {
        let number = |value: &Value| value.as_u64().unwrap_or(0);
        let projects = issues["projects"].as_array().map_or(&[][..], Vec::as_slice);
        let total_of = |short_name: &str| {
            projects
                .iter()
                .find(|project| project["short_name"] == short_name)
                .map_or(0, |project| number(&project["total"]))
        };
        let per_project: Vec<u64> = self
            .projects
            .iter()
            .map(|project| scaled(project.issues, full, asked))
            .collect();
        let mut by_type = vec![0_u64; self.types.len()];
        for (project, planned) in self.projects.iter().zip(per_project) {
            let found: u64 = project
                .places
                .iter()
                .map(|place| total_of(&place.short_name))
                .sum();
            judge.near(
                &format!("the number of issues in {}", project.short_name),
                found,
                planned,
            );
            for place in &project.places {
                judge.near(
                    &format!("the number of issues in {}", place.short_name),
                    total_of(&place.short_name),
                    planned * u64::from(place.percent) / 100,
                );
            }
            for share in self.types_of(project) {
                if let Some(at) = self.types.iter().position(|known| known.name == share.name) {
                    by_type[at] += planned * u64::from(share.percent) / 100;
                }
            }
        }
        for (share, planned) in self.types.iter().zip(by_type) {
            judge.near(
                &format!("the number of issues of the type {}", share.name),
                number(&issues["by_type"][share.name.as_str()]),
                planned,
            );
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use serde_json::json;

    use super::*;
    use crate::sample::Layout;

    /// A dump that has exactly what the plan says for `asked` issues.
    pub(crate) fn made(plan: &IssuePlan, asked: u32) -> Value {
        let full: u32 = plan.projects.iter().map(|project| project.issues).sum();
        let mut projects = Vec::new();
        let mut by_type = serde_json::Map::new();
        for project in &plan.projects {
            let planned = scaled(project.issues, full, asked);
            for place in &project.places {
                projects.push(json!({
                    "short_name": place.short_name,
                    "total": planned * u64::from(place.percent) / 100,
                }));
            }
            for share in plan.types_of(project) {
                let entry = by_type.entry(share.name.clone()).or_insert(json!(0));
                *entry =
                    json!(entry.as_u64().unwrap_or(0) + planned * u64::from(share.percent) / 100);
            }
        }
        let relations = scaled(plan.relations, full, asked);
        json!({ "issues": {
            "planned": asked,
            "projects": projects,
            "by_type": by_type,
            "sub_issues": 40,
            "relations": {
                "blocks": relations / 2,
                "duplicate of": relations / 6,
                "causes": relations / 6,
                "related to": relations - relations / 2 - 2 * (relations / 6),
            },
            "longest_blocks_chain": 5,
            "blocks_circles": 1,
            "tags_on_issues": 10,
            "labels_on_issues": 10,
            "own_field_values": { "Severity": 3, "Platforms": 3, "Customer": 3 },
            "comments": scaled(plan.comments, full, asked),
            "edited_comments": 5,
            "edit_locks": 1,
            "drafts": 1,
            "work": made_work(plan, asked, full),
        }})
    }

    /// The part `work` of a dump that has what the plan says.
    fn made_work(plan: &IssuePlan, asked: u32, full: u32) -> Value {
        let work = &plan.work;
        let counts = work.counts(asked, full);
        let pictures = counts.files * u64::from(work.files.pictures_percent) / 100;
        json!({
            "workflows": [
                { "name": "Simple", "statuses": 3, "changes": 6, "asks": 0 },
                { "name": "Bug", "statuses": 5, "changes": 10, "asks": 0 },
                { "name": "Development", "statuses": 6, "changes": 12, "asks": 0 },
                { "name": work.own_workflow.name, "statuses": 3, "changes": 6, "asks": 1 },
            ],
            "own_workflow_projects": [work.own_workflow.project],
            "files": {
                "total": counts.files,
                "pictures": pictures,
                "pdfs": counts.files - pictures,
                "on_comments": counts.files / 5,
                "previews_ready": counts.files,
                "without_preview": 0,
            },
            "notifications": {
                "total": 100,
                "read": 40,
                "by_kind": { "assigned": 30, "followed_activity": 50 },
                "to": { "viewer": 30, "team_member": 70, "guest": 5 },
                "to_guests_about_others": 0,
            },
            "reports": {
                "contributor": counts.contributor_reports,
                "guest": counts.guest_reports,
            },
            "status_changes": { "team_member": counts.team_status_changes },
            "own_changes": { "contributor": counts.contributor_changes },
            "follows": { "viewer": counts.viewer_follows },
        })
    }

    #[test]
    fn the_judge_accepts_what_the_plan_says_for_every_size_of_sample() {
        let plan = Layout::bundled().unwrap().issues;
        for asked in [plan.total, plan.small_total, 200] {
            let dump = made(&plan, asked);
            assert_eq!(
                plan.problems(asked, "run", &dump),
                Vec::<String>::new(),
                "{asked}"
            );
        }
    }

    #[test]
    fn a_count_far_from_the_plan_is_named() {
        let plan = Layout::bundled().unwrap().issues;
        let asked = plan.small_total;
        let ok = made(&plan, asked);

        let mut dump = ok.clone();
        dump["issues"]["projects"][1]["total"] = json!(1);
        let problems = plan.problems(asked, "sqlite-1", &dump);
        assert!(
            problems
                .iter()
                .any(|p| p.contains("sqlite-1") && p.contains("IOS")),
            "{problems:?}"
        );

        let mut dump = ok.clone();
        dump["issues"]["by_type"]["Bug"] = json!(5);
        assert!(
            plan.problems(asked, "run", &dump)
                .iter()
                .any(|p| p.contains("type Bug"))
        );

        let mut dump = ok.clone();
        dump["issues"]["comments"] = json!(3);
        assert!(
            plan.problems(asked, "run", &dump)
                .iter()
                .any(|p| p.contains("comments"))
        );

        // A count close to the plan, within the tolerance, passes.
        let mut dump = ok;
        let planned = dump["issues"]["comments"].as_u64().unwrap();
        dump["issues"]["comments"] = json!(planned + planned / 20);
        assert!(plan.problems(asked, "run", &dump).is_empty());
    }

    #[test]
    fn what_the_plan_has_has_to_be_there() {
        let plan = Layout::bundled().unwrap().issues;
        let asked = plan.small_total;
        let ok = made(&plan, asked);
        for (member, bad, word) in [
            ("longest_blocks_chain", json!(3), "chain"),
            ("blocks_circles", json!(0), "circles"),
            ("blocks_circles", json!(2), "circles"),
            ("sub_issues", json!(0), "sub-issues"),
            ("edited_comments", json!(0), "edited"),
            ("edit_locks", json!(0), "lock"),
            ("drafts", json!(2), "draft"),
            ("planned", json!(7), "asked"),
        ] {
            let mut dump = ok.clone();
            dump["issues"][member] = bad;
            let problems = plan.problems(asked, "run", &dump);
            assert!(
                problems.iter().any(|p| p.contains(word)),
                "{member}: {problems:?}"
            );
        }
        let mut dump = ok.clone();
        dump["issues"]["own_field_values"] = json!({ "Severity": 3 });
        assert!(
            plan.problems(asked, "run", &dump)
                .iter()
                .any(|p| p.contains("Platforms"))
        );
        let mut dump = ok;
        dump["issues"]["relations"] = json!({ "blocks": plan.relations });
        assert!(
            plan.problems(asked, "run", &dump)
                .iter()
                .any(|p| p.contains("every kind"))
        );
    }
}
