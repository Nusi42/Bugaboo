//! What TEST-SAMPLE-DATA holds AI and agents of the sample against (M7 WP-13): the part
//! `issues.work.ai` of the dump, judged by [`AiPlan::judge`] as part of
//! [`WorkPlan::judge`](crate::sample::WorkPlan). The rules are the ones of ADR-0029: no agent holds
//! a role or is *Assigned to* anything, no agent works on an issue its person cannot change, every
//! standing rule runs out within [`MOST_RULE_DAYS`] days and never names a people field, and every
//! mark of AI-written text matches the event that wrote it.

use bb_core::agent_work::{AgentWorkState, MOST_RULE_DAYS};
use serde_json::Value;

use crate::sample::AiPlan;
use crate::sample_issues::Judge;

impl AiPlan {
    /// Judges the part `ai` of the dump.
    pub(crate) fn judge(&self, judge: &mut Judge<'_>, ai: &Value) {
        let number = |value: &Value| value.as_u64().unwrap_or(0);
        let names = |value: &Value| -> Vec<String> {
            value
                .as_array()
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .filter_map(|name| name.as_str().map(str::to_owned))
                .collect()
        };
        judge.exactly("AI switched on", u64::from(ai["enabled"] == true), 1);
        judge.exactly(
            "agreements confirmed by an administrator",
            u64::from(ai["agreement_confirmed"] == true),
            1,
        );
        for (what, found, planned) in [
            ("projects with AI on", &ai["projects_on"], &self.projects_on),
            (
                "projects with AI off",
                &ai["projects_off"],
                &self.projects_off,
            ),
            (
                "projects never sent to AI",
                &ai["never_sent_projects"],
                &self.never_sent_projects,
            ),
            (
                "fields never sent to AI",
                &ai["never_sent_fields"],
                &self.never_sent_fields,
            ),
        ] {
            let mut planned_sorted = planned.clone();
            planned_sorted.sort();
            if names(found) != planned_sorted {
                judge.problem(&format!("the {what} are {found}, the plan has {planned:?}"));
            }
        }
        self.judge_agents(judge, ai);
        Self::judge_work(judge, &ai["work"]);
        self.judge_rules(judge, &ai["rules"]);
        judge.present("AI apps allowed by a Contributor", number(&ai["apps"]));
        let marks = &ai["marks"];
        judge.present("marks of text written by an agent", number(&marks["agent"]));
        judge.exactly(
            "marks that do not match the event that wrote the text",
            number(&marks["agent"])
                - number(&marks["matching_events"]).min(number(&marks["agent"])),
            0,
        );
    }

    /// The agents: the plan's names, all of one person, each with a key, none with a role.
    fn judge_agents(&self, judge: &mut Judge<'_>, ai: &Value) {
        let number = |value: &Value| value.as_u64().unwrap_or(0);
        let agents = ai["agents"].as_array().map_or(&[][..], Vec::as_slice);
        let found: Vec<&str> = agents
            .iter()
            .filter_map(|agent| agent["name"].as_str())
            .collect();
        let planned: Vec<&str> = self.agents.iter().map(String::as_str).collect();
        if found != planned {
            judge.problem(&format!(
                "the agents are {found:?}, the plan has {planned:?}"
            ));
        }
        for agent in agents {
            let name = agent["name"].as_str().unwrap_or_default();
            if agent["person"] != self.person.as_str() {
                judge.problem(&format!(
                    "the agent {name} does not work for {}",
                    self.person
                ));
            }
            judge.present(&format!("key of the agent {name}"), number(&agent["keys"]));
        }
        judge.exactly("agents with a role", number(&ai["agents_with_a_role"]), 0);
        judge.exactly(
            "agents that are Assigned to an issue",
            number(&ai["agents_assigned"]),
            0,
        );
    }

    /// The agent work: every state is there, with a question, a plan, an answer, a sent-back
    /// attempt and an accepted one, and none is on an issue the person cannot change.
    fn judge_work(judge: &mut Judge<'_>, work: &Value) {
        let number = |value: &Value| value.as_u64().unwrap_or(0);
        for state in AgentWorkState::ALL {
            judge.present(
                &format!("agent work in the state {}", state.code()),
                number(&work["by_state"][state.code()]),
            );
        }
        for (what, key) in [
            (
                "agent work waiting for you with a question",
                "waiting_with_question",
            ),
            (
                "agent work waiting for you with a plan",
                "waiting_with_plan",
            ),
            ("questions that were answered", "answered_questions"),
            ("attempts that were sent back", "sent_back"),
            ("attempts that were accepted", "accepted"),
            ("attempts that were thrown away", "thrown_away"),
            ("results attached to attempts", "results"),
        ] {
            judge.present(what, number(&work[key]));
        }
        judge.exactly(
            "pieces of agent work on an issue its person cannot change",
            number(&work["on_issues_the_person_cannot_change"]),
            0,
        );
    }

    /// The standing rules: the one of the plan, proposing only, ending within the limit.
    fn judge_rules(&self, judge: &mut Judge<'_>, rules: &Value) {
        let number = |value: &Value| value.as_u64().unwrap_or(0);
        let rules = rules.as_array().map_or(&[][..], Vec::as_slice);
        judge.exactly("standing rules", rules.len() as u64, 1);
        for rule in rules {
            if rule["search"] != self.rule.search.as_str() {
                judge.problem(&format!("the standing rule runs over {}", rule["search"]));
            }
            let ahead = rule["days_ahead"].as_i64().unwrap_or(-1);
            if !(0..=MOST_RULE_DAYS).contains(&ahead) {
                judge.problem(&format!(
                    "a standing rule runs out in {ahead} days, the most is {MOST_RULE_DAYS}"
                ));
            }
            judge.exactly(
                "standing rules that name a field of people",
                u64::from(rule["names_a_people_field"] == true),
                0,
            );
            judge.exactly(
                "fields a standing rule may change",
                rule["fields"].as_array().map_or(0, Vec::len) as u64,
                0,
            );
            judge.present("matches of the standing rule", number(&rule["matches"]));
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
    fn the_ai_of_the_plan_is_accepted() {
        let (plan, asked, dump) = ok();
        assert_eq!(plan.problems(asked, "run", &dump), Vec::<String>::new());
    }

    #[test]
    fn what_breaks_the_rules_of_agents_is_named() {
        let (plan, asked, ok) = ok();
        for (path, bad, word) in [
            ("enabled", json!(false), "AI switched on"),
            ("agreement_confirmed", json!(false), "agreements"),
            ("projects_on", json!(["OPS"]), "projects with AI on"),
            ("never_sent_projects", json!([]), "never sent to AI"),
            ("agents", json!([]), "the agents are"),
            ("agents_with_a_role", json!(1), "agents with a role"),
            ("agents_assigned", json!(1), "Assigned to"),
            ("work.by_state.stopped", json!(0), "state stopped"),
            ("work.waiting_with_plan", json!(0), "with a plan"),
            ("work.sent_back", json!(0), "sent back"),
            (
                "work.on_issues_the_person_cannot_change",
                json!(1),
                "cannot change",
            ),
            ("rules", json!([]), "standing rules"),
            ("rules.0.days_ahead", json!(91), "runs out in 91 days"),
            (
                "rules.0.names_a_people_field",
                json!(true),
                "field of people",
            ),
            ("rules.0.fields", json!(["priority"]), "may change"),
            ("apps", json!(0), "AI apps"),
            ("marks.matching_events", json!(3), "do not match"),
        ] {
            let mut dump = ok.clone();
            let mut at = &mut dump["issues"]["work"]["ai"];
            for part in path.split('.') {
                at = match part.parse::<usize>() {
                    Ok(index) => &mut at[index],
                    Err(_) => &mut at[part],
                };
            }
            *at = bad;
            let problems = plan.problems(asked, "run", &dump);
            assert!(
                problems.iter().any(|p| p.contains(word)),
                "{path}: {problems:?}"
            );
        }
    }
}
