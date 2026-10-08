//! META-NEW-MODULE: every crate, package and top-level folder is declared in the module map
//! (architecture §4, §10).

use std::collections::BTreeSet;

use super::arch_map_sync;
use super::{Check, Context, Finding};
use crate::docs::building_blocks::{Layout, parse_module_map};
use crate::docs::source::BUILDING_BLOCKS;
use crate::source::read_text;

pub(crate) struct MetaNewModule;

impl Check for MetaNewModule {
    fn id(&self) -> &'static str {
        "META-NEW-MODULE"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let root = context.repo.root();
        let md = read_text(context.repo, BUILDING_BLOCKS)?;
        let rust = arch_map_sync::rust_members(root)?;
        let npm = arch_map_sync::npm_members(root)?;
        let folders = top_level_folders(&context.repo.all_files()?);
        Ok(evaluate(&md, &rust, npm.as_ref(), &folders))
    }
}

/// The top-level folders that hold project files. Folders whose name starts with a dot are tool
/// settings (`.cargo`, `.claude`, `.github`), not modules.
fn top_level_folders(files: &[String]) -> BTreeSet<String> {
    files
        .iter()
        .filter_map(|file| file.split_once('/'))
        .map(|(folder, _)| folder)
        .filter(|folder| !folder.starts_with('.'))
        .map(str::to_owned)
        .collect()
}

fn evaluate(
    md: &str,
    rust: &BTreeSet<String>,
    npm: Option<&BTreeSet<String>>,
    folders: &BTreeSet<String>,
) -> Vec<Finding> {
    let Some(layout) = Layout::parse(md) else {
        return vec![Finding::at(
            BUILDING_BLOCKS,
            "There is no repository layout block under `## Repository layout`.",
        )];
    };
    let module_map = match parse_module_map(md) {
        Ok(map) => map,
        Err(error) => return vec![Finding::at(BUILDING_BLOCKS, format!("{error:#}"))],
    };
    let mut findings = arch_map_sync::missing_from_layout(&layout, rust, npm);
    for member in rust {
        if let Some(name) = member.strip_prefix("crates/")
            && !module_map.contains_key(name)
        {
            findings.push(Finding::at(
                BUILDING_BLOCKS,
                format!("The crate `{name}` is not in the module map diagram. Add it with its dependency rules."),
            ));
        }
    }
    let listed = layout.top_level();
    for folder in folders {
        if !listed.contains(folder) {
            findings.push(Finding::at(
                BUILDING_BLOCKS,
                format!("The top-level folder `{folder}/` is not in the repository layout. Declare it there before adding files."),
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs::building_blocks::fixtures::{LAYOUT, set};

    #[test]
    fn good_example() {
        let rust = set(&["crates/bb-api", "crates/bb-core", "xtask"]);
        let folders = set(&["crates", "design", "web", "xtask"]);
        assert!(evaluate(LAYOUT, &rust, None, &folders).is_empty());
    }

    #[test]
    fn bad_example() {
        let rust = set(&["crates/bb-api", "crates/bb-core", "crates/bb-new", "xtask"]);
        let npm = set(&["web/app", "web/packages/ui", "web/packages/new"]);
        let folders = set(&["crates", "tools"]);
        let findings = evaluate(LAYOUT, &rust, Some(&npm), &folders);
        let text: Vec<&str> = findings.iter().map(|f| f.message.as_str()).collect();
        let has = |needle: &str| text.iter().any(|message| message.contains(needle));
        assert!(has("`crates/bb-new` is in the workspace"));
        assert!(has("`bb-new` is not in the module map"));
        assert!(has("`web/packages/new` is in the npm workspaces"));
        assert!(has("`tools/` is not in the repository layout"));
    }

    #[test]
    fn dot_folders_are_not_modules() {
        let files = vec![
            ".cargo/config.toml".to_owned(),
            "docs/a.md".to_owned(),
            "Cargo.toml".to_owned(),
        ];
        assert_eq!(top_level_folders(&files), set(&["docs"]));
    }
}
