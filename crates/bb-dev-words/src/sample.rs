//! `tools/dev/sample.json`: where the sample organization is made, what its files are called and
//! what its groups, projects and custom roles are called. `xtask` (which makes the folders and
//! checks the dump), the program `bb-sample` (which makes the organization) and the end-to-end
//! tests (`web/app/e2e/support/sample.ts`) read the same file; nothing else names a folder, a
//! file, the variable, a group, a project or a role of the sample.

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;

use crate::sample_large::LargePlan;

/// The text of the file, built into the program that needs it without a file on disk.
const BUNDLED: &str = include_str!("../../../tools/dev/sample.json");

/// The file is not usable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutError(String);

impl LayoutError {
    pub(crate) const fn new(text: String) -> Self {
        Self(text)
    }
}

impl fmt::Display for LayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for LayoutError {}

/// The whole file. The folders are relative to the work folder and use `/` as separator.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    /// Where the sample is made on SQLite when no folder is given.
    pub out: String,
    /// The same on PostgreSQL.
    pub out_postgres: String,
    /// Where `cargo xtask dev --sample` keeps its sample.
    pub dev_out: String,
    /// Where `--check` makes its runs, one folder each.
    pub check_out: String,
    /// The data folder inside the output folder.
    pub data_folder: String,
    /// The file with the passwords, inside the output folder: the only one that holds a secret.
    pub sign_in_file: String,
    /// The canonical dump, inside the output folder.
    pub dump_file: String,
    /// The file that marks a folder as made by this tool.
    pub marker_file: String,
    /// The key in the marker file that holds the seed.
    pub marker_seed_key: String,
    /// The key in the marker file that holds the moment the sample was made (seconds since 1970).
    pub marker_start_key: String,
    /// The environment variable that holds the address of the empty PostgreSQL database of the
    /// program (an address with a password does not belong on a command line, M1-F62).
    pub database_url_variable: String,
    /// What the groups, projects and custom roles of the organization are called.
    pub organization: OrganizationNames,
    /// What the organization holds besides the people: the issues and what belongs to them.
    pub issues: IssuePlan,
    /// The large dataset of PERF-01: the same kinds of things at a larger scale.
    pub large: LargePlan,
}

/// What the sample holds in issues (architecture §11, *Projects*): the counts of the full sample,
/// which are scaled to the number asked for.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuePlan {
    /// How many issues the full sample has (the demo and the browser tests).
    pub total: u32,
    /// How many the smaller sample has that `--check` makes four times and `cargo xtask dev --sample`
    /// makes for development.
    pub small_total: u32,
    /// How many months back the first issue was made.
    pub months: u32,
    /// How far a made count may be from the plan, in percent of the plan.
    pub tolerance_percent: u32,
    /// The share of each issue type, in percent (together 100).
    pub types: Vec<TypeShare>,
    /// The issues of each top-level project, and how they are spread over its places.
    pub projects: Vec<ProjectIssues>,
    /// The own fields to make.
    pub fields: Vec<FieldPlan>,
    /// The groups of labels with their labels.
    pub label_groups: Vec<LabelGroupPlan>,
    /// The labels that are in no group.
    pub labels: Vec<String>,
    /// The tags issues carry.
    pub tags: Vec<String>,
    /// How many relations the full sample has.
    pub relations: u32,
    /// How many comments the full sample has.
    pub comments: u32,
    /// How many of a hundred comments were edited.
    pub edited_comments_percent: u32,
    /// How the people of the sample work together: workflows, files, notifications.
    pub work: WorkPlan,
}

/// How the people of the sample work together (M4 WP-10): the workflow a project has of its own,
/// the files on issues and comments, and what Contributors, Guests and Viewers do. The counts are
/// those of the full sample and are scaled like the issues.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkPlan {
    /// The project that has a workflow of its own, and what the workflow is called.
    pub own_workflow: OwnWorkflow,
    /// How many files issues and comments carry.
    pub files: FilesPlan,
    /// Issues that Contributors and Guests reported themselves.
    pub reports: ReportsPlan,
    /// How many status changes the Team members make in the last days, under the workflows.
    pub team_status_changes: u32,
    /// How many changes Contributors make to the issues they reported (a Contributor can change
    /// the title and the tags of their own issues, not the status).
    pub contributor_changes: u32,
    /// How many issues the Viewers follow together (they only read and follow).
    pub viewer_follows: u32,
    /// How many of a hundred notifications of a Viewer or a Guest were read.
    pub read_percent: u32,
    /// The searches people of each role saved, some shared with groups and projects (QRY-06).
    pub saved_searches: Vec<SavedSearchPlan>,
    /// The names of the own fields that Contributors may not read, so a search that reads one
    /// never reaches them.
    pub hidden_from_contributors: Vec<String>,
    /// The working hours, time entries, estimates and timers (M6, TIME-01 to TIME-13).
    pub time: TimePlan,
    /// AI, agents and what they did (M7, WP-13).
    pub ai: AiPlan,
}

/// AI and agents in the sample (M7 part 6): how AI is set up, which agents one person has and what
/// the hand-overs, the questions and the standing rule say. The counts of hand-overs are those of the
/// states agent work can be in, so they are not scaled like the issues.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiPlan {
    /// What a line of `sign-in.txt` starts with (before a tab) when it holds the key of an agent.
    pub agent_key_mark: String,
    /// The address of the provider: a made-up one that never answers.
    pub address: String,
    /// The model.
    pub model: String,
    /// The short names of the projects AI is switched on for.
    pub projects_on: Vec<String>,
    /// The short names of the projects AI is switched off for.
    pub projects_off: Vec<String>,
    /// The short names of the projects marked *Never sent to AI*.
    pub never_sent_projects: Vec<String>,
    /// The codes of the built-in fields marked *Never sent to AI*.
    pub never_sent_fields: Vec<String>,
    /// The name of the Team member the agents work for.
    pub person: String,
    /// The short name of the project the agents work in.
    pub project: String,
    /// The names of the two agents, the first one takes most of the work.
    pub agents: Vec<String>,
    /// The note of the brief of every hand-over.
    pub note: String,
    /// The question an agent asks.
    pub question: String,
    /// The answer to it.
    pub answer: String,
    /// The plan an agent shows for approval.
    pub plan: String,
    /// What the reviewer writes when sending an attempt back.
    pub sent_back_note: String,
    /// Why an agent gave up.
    pub gave_up: String,
    /// The name of the check an agent attaches.
    pub check_label: String,
    /// The standing rule over new bugs.
    pub rule: RulePlan,
    /// The AI app a Contributor allowed.
    pub app: AppPlan,
}

/// The standing rule of the sample: it proposes and changes nothing.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulePlan {
    /// The name of the saved search the rule runs over.
    pub search: String,
    /// The search text.
    pub text: String,
    /// After how many days the rule runs out.
    pub days: i64,
}

/// An AI app a Contributor allowed.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPlan {
    /// What the app is called.
    pub name: String,
    /// The address the app is sent back to after the person allowed it.
    pub return_address: String,
}

/// A choice of *Cost center* the sample adds: its number and its name.
#[derive(Clone, Debug, Deserialize)]
pub struct CostCenterPlan {
    /// The number, digits (`4100`).
    pub number: String,
    /// The name (`Product development`).
    pub name: String,
}

/// The working hours and the time on issues of the sample: the weeks the people recorded, how many
/// time entries they booked, which days broke a rule or were changed afterwards, and the timers
/// left running. The counts are those of the full sample and are scaled like the issues.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimePlan {
    /// How many weeks of working days are recorded, counted back from the end of the sample.
    pub weeks: u32,
    /// How many time entries are booked in all.
    pub entries: u32,
    /// How many employees a small sample has at least: a sample of few issues records the hours of
    /// the first few employees (the people of the demos first), so it stays quick to make.
    pub employees_at_least: u32,
    /// How many of a hundred working days a person does not work (leave).
    pub leave_percent: u32,
    /// How many of a hundred worked days break a working-time rule (a day that is too long, or a
    /// long day without a break).
    pub rule_breaking_percent: u32,
    /// How many days people changed afterwards, with a reason, on their own.
    pub changed_by_person: u32,
    /// How many days somebody with the role *HR* changed, with a reason.
    pub changed_by_hr: u32,
    /// How many timers are left running at the end.
    pub running_timers: u32,
    /// How many of a hundred issues have an *Estimated time*.
    pub estimated_percent: u32,
    /// The choices of *Cost center* the sample adds, each a number and a name.
    pub cost_centers: Vec<CostCenterPlan>,
    /// The reasons people give for a change of their own day.
    pub person_reasons: Vec<String>,
    /// The reasons *HR* gives for a change of the day of somebody else.
    pub hr_reasons: Vec<String>,
}

/// A saved search of the sample: who made it, whom it is shared with, and the project that shows
/// it as its list.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSearchPlan {
    /// What it is called.
    pub name: String,
    /// The search text.
    pub text: String,
    /// The starting role of the first person with that role, who makes it (its code).
    pub maker: String,
    /// The keys of the groups it is shared with.
    pub groups: Vec<String>,
    /// The short names of the projects it is shared with.
    pub projects: Vec<String>,
    /// The short name of the project that shows it as its list; it must be one of `projects`.
    pub project_list: Option<String>,
}

/// The workflow a project has of its own: the starting workflow with a status change that asks
/// for the person who does the work (*Ask for*).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnWorkflow {
    /// The short name of the project.
    pub project: String,
    /// What the workflow is called.
    pub name: String,
}

/// The files of the sample.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilesPlan {
    /// How many files the full sample has.
    pub total: u32,
    /// How many of a hundred files are pictures (the rest are PDFs).
    pub pictures_percent: u32,
    /// How many of a hundred files belong to a comment (the rest to an issue).
    pub on_comments_percent: u32,
}

/// The issues Contributors and Guests report.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportsPlan {
    /// Together, all Contributors.
    pub contributors: u32,
    /// Together, all Guests.
    pub guests: u32,
}

/// The share of an issue type.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeShare {
    /// The name of the starting issue type.
    pub name: String,
    /// Its share, in percent.
    pub percent: u32,
}

/// The issues of one top-level project.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectIssues {
    /// The short name of the project.
    pub short_name: String,
    /// The list of things (`[things]` of `issues.toml`) the titles of its issues are made from.
    pub area: String,
    /// How many issues the project and its sub-projects have in the full sample.
    pub issues: u32,
    /// How its issues are given their IDs: `random` or `counting`.
    pub id_mode: String,
    /// Where the issues are: the project itself and its sub-projects, with their shares.
    pub places: Vec<Place>,
    /// Another share of the types than the one of the whole sample, if the project has one.
    #[serde(default)]
    pub types: Option<Vec<TypeShare>>,
}

/// A project that holds issues, and its share of the issues of its top-level project.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    /// The short name of the project.
    pub short_name: String,
    /// Its share, in percent.
    pub percent: u32,
}

/// An own field to make.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldPlan {
    /// What it is called.
    pub name: String,
    /// The kind of information, as its word (`pick-one`, `pick-several`, `text`).
    pub kind: String,
    /// The names of the issue types it is for; none means all.
    pub issue_types: Vec<String>,
    /// The short names of the projects it is used in; none means all.
    pub projects: Vec<String>,
    /// The choices of a field that has them.
    pub choices: Vec<String>,
}

/// A group of labels.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelGroupPlan {
    /// What it is called.
    pub name: String,
    /// Whether an issue has at most one label of the group.
    pub pick_one: bool,
    /// The labels of the group.
    pub labels: Vec<String>,
}

/// What the people of the sample read as the names of its groups, projects and custom roles. The
/// code names a group or a role by its key and a project by its short name.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationNames {
    /// The groups: key, name.
    pub groups: BTreeMap<String, String>,
    /// The custom roles: key, name.
    pub roles: BTreeMap<String, String>,
    /// The projects of the top level, with their sub-projects.
    pub projects: Vec<ProjectNames>,
}

/// A project of the sample and the sub-projects inside it.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectNames {
    /// The letters of its address.
    pub short_name: String,
    /// What it is called.
    pub name: String,
    /// The projects inside it.
    pub sub_projects: Vec<SubProjectNames>,
}

/// A sub-project of the sample.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubProjectNames {
    /// The letters of its address.
    pub short_name: String,
    /// What it is called.
    pub name: String,
}

impl OrganizationNames {
    /// The name of the group with this key.
    ///
    /// # Errors
    /// [`LayoutError`] when the file has no such group.
    pub fn group(&self, key: &str) -> Result<&str, LayoutError> {
        named(&self.groups, "groups", key)
    }

    /// The name of the custom role with this key.
    ///
    /// # Errors
    /// [`LayoutError`] when the file has no such role.
    pub fn role(&self, key: &str) -> Result<&str, LayoutError> {
        named(&self.roles, "roles", key)
    }
}

fn named<'a>(
    names: &'a BTreeMap<String, String>,
    kind: &str,
    key: &str,
) -> Result<&'a str, LayoutError> {
    names
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| LayoutError(format!("{kind}.{key} is missing")))
}

impl IssuePlan {
    /// Whether the plan adds up: shares of 100, places that are projects of the layout.
    fn check(&self) -> Result<(), LayoutError> {
        let sum = |shares: &[TypeShare]| shares.iter().map(|share| share.percent).sum::<u32>();
        let bad = |what: &str| Err(LayoutError(format!("issues: {what}")));
        if sum(&self.types) != 100 {
            return bad("the shares of the types are not 100");
        }
        for project in &self.projects {
            if project
                .places
                .iter()
                .map(|place| place.percent)
                .sum::<u32>()
                != 100
            {
                return bad("the places of a project do not make 100");
            }
            if project
                .types
                .as_ref()
                .is_some_and(|types| sum(types) != 100)
            {
                return bad("the shares of the types of a project are not 100");
            }
        }
        if self.total == 0 || self.small_total == 0 || self.months == 0 {
            return bad("a count is 0");
        }
        Ok(())
    }
}

impl Layout {
    /// The layout that is part of this program.
    ///
    /// # Errors
    /// [`LayoutError`] when the file is not usable (a test keeps it usable).
    pub fn bundled() -> Result<Self, LayoutError> {
        Self::parse(BUNDLED)
    }

    /// A layout from the text of a `sample.json` file.
    ///
    /// # Errors
    /// [`LayoutError`] when the text lacks a name or has an empty one.
    pub fn parse(text: &str) -> Result<Self, LayoutError> {
        let layout: Self =
            serde_json::from_str(text).map_err(|error| LayoutError(error.to_string()))?;
        layout.large.check()?;
        let names = [
            &layout.out,
            &layout.out_postgres,
            &layout.dev_out,
            &layout.check_out,
            &layout.data_folder,
            &layout.sign_in_file,
            &layout.dump_file,
            &layout.marker_file,
            &layout.database_url_variable,
        ];
        let organization = &layout.organization;
        let projects = organization.projects.iter().flat_map(|project| {
            std::iter::once((&project.short_name, &project.name)).chain(
                project
                    .sub_projects
                    .iter()
                    .map(|sub| (&sub.short_name, &sub.name)),
            )
        });
        layout.issues.check()?;
        if names.iter().any(|name| name.trim().is_empty())
            || organization
                .groups
                .values()
                .chain(organization.roles.values())
                .any(|name| name.trim().is_empty())
            || projects
                .into_iter()
                .any(|(short, name)| short.trim().is_empty() || name.trim().is_empty())
        {
            return Err(LayoutError("a name is empty".to_owned()));
        }
        Ok(layout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_layout_is_usable() {
        let layout = Layout::bundled().unwrap();
        assert!(!layout.sign_in_file.is_empty());
        assert!(layout.database_url_variable.starts_with("BB_SAMPLE_"));
        assert_eq!(layout.organization.projects.len(), 4);
        assert_eq!(layout.issues.projects.len(), 4);
        let planned: u32 = layout
            .issues
            .projects
            .iter()
            .map(|project| project.issues)
            .sum();
        assert_eq!(planned, layout.issues.total);
        assert_eq!(layout.organization.group("backend").unwrap(), "Backend");
        assert!(layout.organization.role("no-such-role").is_err());
    }

    #[test]
    fn a_layout_with_a_missing_or_empty_name_is_refused() {
        assert!(Layout::parse("{}").is_err());
        let layout = Layout::bundled().unwrap();
        let empty = BUNDLED.replace(&format!("\"{}\"", layout.sign_in_file), "\" \"");
        assert!(Layout::parse(&empty).is_err());
    }
}
