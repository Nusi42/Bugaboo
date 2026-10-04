//! PLAN-TRACE: every feature that is planned appears in exactly one milestone, and work packages
//! only name features that exist (docs/04-milestones.md).

use std::collections::{HashMap, HashSet};

use super::{Check, Context, Finding};
use crate::docs::source::{
    FEATURE_LIST, MILESTONE_PACKAGES_DIR, MILESTONES, read_text, sorted_files,
};
use crate::docs::text;

/// The last milestone that delivers MVP ("M") features; later ones deliver 1.0 ("V1") features.
const LAST_MVP_MILESTONE: u32 = 9;

pub(crate) struct PlanTrace;

impl Check for PlanTrace {
    fn id(&self) -> &'static str {
        "PLAN-TRACE"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        let features = read_text(repo, FEATURE_LIST)?;
        let milestones = read_text(repo, MILESTONES)?;
        let mut packages = Vec::new();
        for folder in sorted_files(repo, MILESTONE_PACKAGES_DIR, |name| !name.contains('.'))? {
            let folder = format!("{MILESTONE_PACKAGES_DIR}/{folder}");
            for name in sorted_files(repo, &folder, |name| text::has_extension(name, "md"))? {
                let path = format!("{folder}/{name}");
                let text = read_text(repo, &path)?;
                packages.push((path, text));
            }
        }
        Ok(evaluate(&features, &milestones, &packages))
    }
}

/// The ID prefixes of the feature list's sections: `## 1. Security & access control (SEC)`.
/// Only IDs with one of these prefixes are feature IDs; `CWE-79` or `ADR-0014` are not.
fn feature_prefixes(md: &str) -> HashSet<String> {
    md.lines()
        .filter(|line| line.starts_with("## "))
        .filter_map(|line| {
            let inner = line.trim_end().strip_suffix(')')?.rsplit_once('(')?.1;
            inner
                .bytes()
                .all(|b| b.is_ascii_uppercase())
                .then(|| inner.to_owned())
        })
        .collect()
}

fn prefix_of(id: &str) -> &str {
    id.split('-').next().unwrap_or(id)
}

/// The feature IDs in `text` that start with a known section prefix.
fn feature_ids_in(text: &str, prefixes: &HashSet<String>) -> Vec<String> {
    text::feature_ids(text)
        .into_iter()
        .filter(|id| prefixes.contains(prefix_of(id)))
        .collect()
}

struct Feature {
    id: String,
    /// `M`, `V1` or `L`.
    tier: String,
}

/// Feature lines like `- **ISS-01 (M)** — …`, and the lines that look like one but do not parse.
fn parse_features(md: &str) -> (Vec<Feature>, Vec<(usize, String)>) {
    let mut features = Vec::new();
    let mut unparsed = Vec::new();
    for (index, line) in md.lines().enumerate() {
        let Some(rest) = line.trim_start().strip_prefix("- **") else {
            continue;
        };
        let first = rest.split([' ', '*']).next().unwrap_or("");
        let looks_like_id = first.split_once('-').is_some_and(|(prefix, number)| {
            prefix.len() >= 2
                && prefix.bytes().all(|b| b.is_ascii_uppercase())
                && number.starts_with(|c: char| c.is_ascii_digit())
        });
        if !looks_like_id {
            continue;
        }
        let tier = rest
            .strip_prefix(first)
            .and_then(|after| after.trim_start().strip_prefix('('))
            .and_then(|after| after.split_once(')'))
            .and_then(|(tier, _)| tier.split(',').next())
            .map(str::trim);
        match tier {
            Some(tier @ ("M" | "V1" | "L")) if text::feature_ids(first) == [first] => {
                features.push(Feature {
                    id: first.to_owned(),
                    tier: tier.to_owned(),
                });
            }
            _ => unparsed.push((index + 1, first.to_owned())),
        }
    }
    (features, unparsed)
}

/// Feature IDs per milestone, from the `## M3 — …` headings and the bullets under them.
fn parse_milestones(md: &str, prefixes: &HashSet<String>) -> Vec<(u32, String)> {
    let mut current = None;
    let mut found = Vec::new();
    for line in md.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            current = rest.strip_prefix('M').and_then(|digits| {
                let end = digits.find(|c: char| !c.is_ascii_digit())?;
                digits[..end].parse().ok()
            });
        } else if let (Some(milestone), Some(rest)) = (current, line.strip_prefix("- `"))
            && let Some((id, _)) = rest.split_once('`')
            && feature_ids_in(id, prefixes) == [id]
        {
            found.push((milestone, id.to_owned()));
        }
    }
    found
}

fn evaluate(feature_list: &str, milestones: &str, packages: &[(String, String)]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let prefixes = feature_prefixes(feature_list);
    let (features, unparsed) = parse_features(feature_list);
    for (line, first) in unparsed {
        findings.push(Finding::at(
            format!("{FEATURE_LIST}:{line}"),
            format!(
                "`{first}` looks like a feature line but cannot be read. Write it as `- **ABC-01 (M)** — …` with tier M, V1 or L."
            ),
        ));
    }
    let mut seen = HashSet::new();
    for feature in &features {
        if !seen.insert(feature.id.as_str()) {
            findings.push(Finding::at(
                &feature.id,
                "This feature ID is defined more than once in the feature list.",
            ));
        }
    }
    let known: HashSet<&str> = features.iter().map(|feature| feature.id.as_str()).collect();
    let scheduled = parse_milestones(milestones, &prefixes);

    let mut milestones_of: HashMap<&str, Vec<u32>> = HashMap::new();
    for (milestone, id) in &scheduled {
        milestones_of.entry(id).or_default().push(*milestone);
    }
    for feature in &features {
        let (id, tier) = (&feature.id, feature.tier.as_str());
        let places = milestones_of
            .get(id.as_str())
            .map_or(&[][..], Vec::as_slice);
        match (tier, places) {
            ("L", []) => {}
            ("L", _) => findings.push(Finding::at(
                id,
                "This is a later (L) feature, but a milestone schedules it. Remove it from the milestone or change its tier.",
            )),
            (_, []) => findings.push(Finding::at(
                id,
                format!("This {tier} feature is in no milestone. Add it to exactly one milestone in {MILESTONES}."),
            )),
            (_, [only]) => {
                if tier == "M" && *only > LAST_MVP_MILESTONE {
                    findings.push(Finding::at(
                        id,
                        format!("This MVP (M) feature is in M{only}; M features belong in M0 to M{LAST_MVP_MILESTONE}."),
                    ));
                }
                if tier == "V1" && *only <= LAST_MVP_MILESTONE {
                    findings.push(Finding::at(
                        id,
                        format!("This 1.0 (V1) feature is in M{only}; V1 features belong in M{} or later.", LAST_MVP_MILESTONE + 1),
                    ));
                }
            }
            (_, many) => {
                let list: Vec<String> = many.iter().map(|n| format!("M{n}")).collect();
                findings.push(Finding::at(
                    id,
                    format!("This feature is in {} milestones ({}); keep it in exactly one.", many.len(), list.join(", ")),
                ));
            }
        }
    }
    for (milestone, id) in &scheduled {
        if !known.contains(id.as_str()) {
            findings.push(Finding::at(
                format!("{MILESTONES} (M{milestone})"),
                format!("`{id}` is not in the feature list."),
            ));
        }
    }
    for (path, text) in packages {
        let ids = feature_ids_in(text, &prefixes);
        for id in ids.iter().filter(|id| !known.contains(id.as_str())) {
            findings.push(Finding::at(
                path,
                format!("`{id}` is not in the feature list. Fix the ID."),
            ));
        }
        let is_package = path
            .rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("WP-"));
        if is_package && ids.is_empty() {
            findings.push(Finding::at(
                path,
                "This work package names no feature ID. Say which features it delivers.",
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEATURES: &str = "\
## 1. Things (AAA)
- **AAA-01 (M)** — one
## 2. More things (BBB)
- **BBB-01 (V1)** — two
  - **BBB-02a (V1, last)** — three
## 3. Later things (CCC)
- **CCC-01 (L)** — later
";

    fn milestones(m1: &str, m10: &str) -> String {
        format!(
            "# T\n\n## M1 — a\n\n**Features**\n\n{m1}\n## M10 — b\n\n{m10}\n## Later\n\n- `CCC-01` x\n"
        )
    }

    fn package(text: &str) -> Vec<(String, String)> {
        vec![("docs/milestones/M1/WP-01-x.md".to_owned(), text.to_owned())]
    }

    fn messages(findings: &[Finding]) -> Vec<String> {
        findings
            .iter()
            .map(|f| format!("{} {}", f.location.clone().unwrap_or_default(), f.message))
            .collect()
    }

    #[test]
    fn good_example() {
        let md = milestones("- `AAA-01` one\n", "- `BBB-01` two\n- `BBB-02a` three\n");
        let package = package("Delivers AAA-01 and BBB-02a, not CWE-79 or ADR-0014.");
        assert!(evaluate(FEATURES, &md, &package).is_empty());
    }

    #[test]
    fn bad_example() {
        let md = milestones(
            "- `AAA-01` one\n- `BBB-01` two\n",
            "- `AAA-01` again\n- `AAA-99` ghost\n",
        );
        let text = messages(&evaluate(FEATURES, &md, &package("Delivers AAA-98.")));
        let has = |needle: &str| text.iter().any(|line| line.contains(needle));
        assert!(has("AAA-01 This feature is in 2 milestones"));
        assert!(has("BBB-01 This 1.0 (V1) feature is in M1"));
        assert!(has("BBB-02a This V1 feature is in no milestone"));
        assert!(has("`AAA-99` is not in the feature list"));
        assert!(has("`AAA-98` is not in the feature list"));
    }

    #[test]
    fn a_work_package_without_feature_ids_fails() {
        let md = milestones("- `AAA-01` one\n", "- `BBB-01` two\n- `BBB-02a` three\n");
        let text = messages(&evaluate(FEATURES, &md, &package("Delivers CWE-79 only.")));
        assert_eq!(text.len(), 1);
        assert!(text[0].contains("names no feature ID"));
    }

    #[test]
    fn mvp_feature_in_a_late_milestone_and_scheduled_later_feature_fail() {
        let md =
            "## M12 — x\n\n- `AAA-01` one\n- `CCC-01` later\n- `BBB-01` two\n- `BBB-02a` three\n";
        let text = messages(&evaluate(FEATURES, md, &[]));
        let has = |needle: &str| text.iter().any(|line| line.contains(needle));
        assert!(has("M features belong in M0 to M9"));
        assert!(has("later (L) feature"));
    }

    #[test]
    fn broken_and_duplicate_feature_lines_are_reported() {
        let features = format!(
            "{FEATURES}- **AAA-02 (X)** — bad tier\n- **AAA-03** — no tier\n- **AAA-01 (M)** — again\n"
        );
        let text = messages(&evaluate(&features, "", &[]));
        let has = |needle: &str| text.iter().any(|line| line.contains(needle));
        assert!(has("`AAA-02` looks like a feature line but cannot be read"));
        assert!(has("`AAA-03` looks like a feature line but cannot be read"));
        assert!(has("defined more than once"));
    }
}
