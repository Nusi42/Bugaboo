//! TEST-VISUAL: every kind of view and a dashboard has its reference pictures, in every built-in
//! theme, on a computer and on a phone, and all of them together stay small (architecture §11,
//! *Visual tests*).
//!
//! The kinds of view are read from the API description (`ViewKind`), the themes from the files of
//! `design/themes/`, and the rest from the settings of the suite (`web/app/e2e/visual/settings.json`:
//! the folder, the kinds beyond the views, the devices and the most megabytes). A picture is named
//! `<kind>-<theme>-<device>.png`. The check fails when a picture is missing, when a picture names
//! no kind, theme and device that exist (a picture left over from a view or a theme that went), and
//! when the pictures together are larger than the limit. The allowed share of differing pixels in
//! the settings is 0: three runs in a row of all pictures on a quiet computer gave the same pixels
//! (architecture §11). The comparison itself runs in the browser
//! tests of `cargo xtask ci` on Windows (`web/app/e2e/visual/`).

use std::collections::BTreeSet;

use anyhow::Context as _;
use serde::Deserialize;
use serde_json::Value;

use super::{Check, Context, Finding, api_spec};
use crate::docs::source::sorted_files;
use crate::generate::theme_file::theme_ids;
use crate::source::read_text;

/// The settings of the suite, which the Playwright configuration and the suite read too.
pub(crate) const SETTINGS_FILE: &str = "web/app/e2e/visual/settings.json";
/// The schema of the API description that names the kinds of view.
const VIEW_KINDS: &str = "ViewKind";
/// The ending of a picture.
const PICTURE_ENDING: &str = ".png";
const BYTES_PER_MEGABYTE: u64 = 1024 * 1024;

/// What the check reads of the settings.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Settings {
    /// The folder of the pictures, relative to the work folder.
    pub(crate) pictures: String,
    /// The pictures beyond the kinds of view (a dashboard).
    kinds: Vec<String>,
    devices: Vec<String>,
    most_megabytes: u64,
}

/// Reads the settings of the suite.
pub(crate) fn settings(repo: &crate::repo::Repo) -> anyhow::Result<Settings> {
    serde_json::from_str(&read_text(repo, SETTINGS_FILE)?)
        .with_context(|| format!("{SETTINGS_FILE} is not valid"))
}

pub(crate) struct TestVisual;

impl Check for TestVisual {
    fn id(&self) -> &'static str {
        "TEST-VISUAL"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        let settings = settings(repo)?;
        let spec = api_spec::load(repo)?;
        let mut kinds = view_kinds(&spec);
        if kinds.is_empty() {
            return Ok(vec![Finding::at(
                crate::generate::api::SPEC_PATH,
                format!("The API description has no values of `{VIEW_KINDS}`, the kinds of view."),
            )]);
        }
        kinds.extend(settings.kinds.iter().cloned());
        let themes = theme_ids(repo)?;
        let pictures = if repo.path(&settings.pictures).is_dir() {
            sorted_files(repo, &settings.pictures, |_| true)?
                .into_iter()
                .map(|name| {
                    let path = repo.path(&format!("{}/{name}", settings.pictures));
                    let bytes = std::fs::metadata(&path)
                        .with_context(|| format!("could not read {}", path.display()))?
                        .len();
                    Ok((name, bytes))
                })
                .collect::<anyhow::Result<Vec<_>>>()?
        } else {
            Vec::new()
        };
        Ok(evaluate(
            &Wanted {
                kinds: &kinds,
                themes: &themes,
                settings: &settings,
            },
            &pictures,
        ))
    }
}

/// The kinds of view of the API description.
fn view_kinds(spec: &Value) -> Vec<String> {
    api_spec::schemas(spec)
        .into_iter()
        .find(|(name, _)| *name == VIEW_KINDS)
        .and_then(|(_, schema)| schema.get("enum"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

/// What pictures there must be.
struct Wanted<'a> {
    kinds: &'a [String],
    themes: &'a [String],
    settings: &'a Settings,
}

impl Wanted<'_> {
    /// The name of every picture there must be.
    fn names(&self) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        for kind in self.kinds {
            for theme in self.themes {
                for device in &self.settings.devices {
                    names.insert(format!("{kind}-{theme}-{device}{PICTURE_ENDING}"));
                }
            }
        }
        names
    }
}

/// The findings for the pictures that are there, each with its size in bytes.
fn evaluate(wanted: &Wanted<'_>, pictures: &[(String, u64)]) -> Vec<Finding> {
    let folder = &wanted.settings.pictures;
    let names = wanted.names();
    let there: BTreeSet<&str> = pictures.iter().map(|(name, _)| name.as_str()).collect();
    let mut findings: Vec<Finding> = names
        .iter()
        .filter(|name| !there.contains(name.as_str()))
        .map(|name| {
            Finding::at(
                format!("{folder}/{name}"),
                "This reference picture is missing: every kind of view and a dashboard is pictured in every built-in theme, on a computer and on a phone. Take it with `cargo xtask visual update` and look at it before it is committed.",
            )
        })
        .collect();
    findings.extend(
        pictures
            .iter()
            .filter(|(name, _)| !names.contains(name))
            .map(|(name, _)| {
                Finding::at(
                    format!("{folder}/{name}"),
                    "This file is no reference picture of a kind of view, a theme and a device that exist (`<kind>-<theme>-<device>.png`): remove it.",
                )
            }),
    );
    let total: u64 = pictures.iter().map(|(_, bytes)| bytes).sum();
    let most = wanted.settings.most_megabytes * BYTES_PER_MEGABYTE;
    if total > most {
        findings.push(Finding::at(
            folder.clone(),
            format!(
                "The reference pictures take {total} bytes together, more than the {} MB of {SETTINGS_FILE}: picture less of each screen, never raise the limit.",
                wanted.settings.most_megabytes
            ),
        ));
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        Settings {
            pictures: "pictures".to_owned(),
            kinds: vec!["dashboard".to_owned()],
            devices: vec!["computer".to_owned(), "phone".to_owned()],
            most_megabytes: 10,
        }
    }

    fn strings(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_owned()).collect()
    }

    /// Every picture of the views `list` and `board` and a dashboard, in two themes, of `bytes` each.
    fn all_pictures(bytes: u64) -> Vec<(String, u64)> {
        let mut pictures = Vec::new();
        for kind in ["list", "board", "dashboard"] {
            for theme in ["dusk", "paper"] {
                for device in ["computer", "phone"] {
                    pictures.push((format!("{kind}-{theme}-{device}.png"), bytes));
                }
            }
        }
        pictures
    }

    fn found(pictures: &[(String, u64)]) -> Vec<Finding> {
        let settings = settings();
        let mut kinds = strings(&["list", "board"]);
        kinds.extend(settings.kinds.iter().cloned());
        evaluate(
            &Wanted {
                kinds: &kinds,
                themes: &strings(&["dusk", "paper"]),
                settings: &settings,
            },
            pictures,
        )
    }

    #[test]
    fn good_example() {
        assert!(found(&all_pictures(1000)).is_empty());
    }

    #[test]
    fn bad_example_a_missing_picture() {
        let mut pictures = all_pictures(1000);
        pictures.retain(|(name, _)| name != "board-paper-phone.png");
        let findings = found(&pictures);
        assert_eq!(findings.len(), 1);
        assert!(
            findings[0]
                .location
                .as_deref()
                .is_some_and(|place| place.ends_with("board-paper-phone.png"))
        );
    }

    #[test]
    fn test_visual_fails_a_view_kind_without_its_pictures() {
        let mut pictures = all_pictures(1000);
        pictures.retain(|(name, _)| !name.starts_with("board-"));
        // Two themes on two devices.
        assert_eq!(found(&pictures).len(), 4);
    }

    #[test]
    fn test_visual_fails_when_the_pictures_exceed_10_mb() {
        let pictures = all_pictures(1000);
        let each = 10 * BYTES_PER_MEGABYTE / u64::try_from(pictures.len()).unwrap() + 1;
        let findings = found(&all_pictures(each));
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("10 MB"));
        let at_the_limit = 10 * BYTES_PER_MEGABYTE / u64::try_from(pictures.len()).unwrap();
        assert!(found(&all_pictures(at_the_limit)).is_empty());
    }

    #[test]
    fn bad_example_a_picture_of_a_theme_that_went() {
        let mut pictures = all_pictures(1000);
        pictures.push(("list-harbor-computer.png".to_owned(), 1000));
        assert_eq!(found(&pictures).len(), 1);
    }

    #[test]
    fn the_kinds_of_view_are_read_from_the_api_description() {
        let spec = serde_json::json!({
            "components": { "schemas": { "ViewKind": { "type": "string", "enum": ["list", "map"] } } }
        });
        assert_eq!(view_kinds(&spec), strings(&["list", "map"]));
    }

    #[test]
    fn the_real_settings_and_views_are_read() {
        let repo = crate::repo::Repo::discover().unwrap();
        let settings = super::settings(&repo).unwrap();
        assert!(settings.devices.contains(&"phone".to_owned()));
        let kinds = view_kinds(&api_spec::load(&repo).unwrap());
        assert!(kinds.contains(&"calendar".to_owned()));
    }
}
