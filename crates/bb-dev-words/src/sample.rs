//! `tools/dev/sample.json`: where the sample organization is made, what its files are called and
//! what its groups, projects and custom roles are called. `xtask` (which makes the folders and
//! checks the dump), the program `bb-sample` (which makes the organization) and the end-to-end
//! tests (`web/app/e2e/support/sample.ts`) read the same file; nothing else names a folder, a
//! file, the variable, a group, a project or a role of the sample.

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;

/// The text of the file, built into the program that needs it without a file on disk.
const BUNDLED: &str = include_str!("../../../tools/dev/sample.json");

/// The file is not usable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutError(String);

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
    /// The environment variable that holds the address of the empty PostgreSQL database of the
    /// program (an address with a password does not belong on a command line, M1-F62).
    pub database_url_variable: String,
    /// What the groups, projects and custom roles of the organization are called.
    pub organization: OrganizationNames,
    /// What the organization holds besides the people: the issues and what belongs to them.
    pub issues: IssuePlan,
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
