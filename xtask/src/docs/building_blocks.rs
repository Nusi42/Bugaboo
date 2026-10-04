//! The architecture chapter "Building blocks" (`docs/architecture/04-building-blocks.md`): the one
//! reader for its repository layout and its module map. ARCH-MAP-SYNC, META-NEW-MODULE and
//! ARCH-DEPS all use it, so the document is parsed in one place.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context as _, bail};

use super::source::BUILDING_BLOCKS;

/// The repository layout block: every path it lists, like `crates/bb-core` or `web/app`.
pub(crate) struct Layout {
    paths: Vec<String>,
}

impl Layout {
    /// Reads the first `text` block under `## Repository layout`.
    pub(crate) fn parse(md: &str) -> Option<Self> {
        let mut lines = md
            .lines()
            .skip_while(|line| line.trim_end() != "## Repository layout")
            .skip_while(|line| !line.starts_with("```"));
        lines.next()?;
        let mut stack: Vec<String> = Vec::new();
        let mut paths = Vec::new();
        for line in lines.take_while(|line| !line.starts_with("```")) {
            let Some(indent) = line.chars().position(|c| !"│├└─ ".contains(c)) else {
                continue;
            };
            let depth = indent / 4;
            let name = line
                .chars()
                .skip(indent)
                .take_while(|c| !c.is_whitespace())
                .collect::<String>();
            if depth == 0 || name.is_empty() {
                continue;
            }
            stack.truncate(depth - 1);
            stack.push(name.trim_end_matches('/').to_owned());
            paths.push(stack.join("/"));
        }
        Some(Self { paths })
    }

    /// The paths directly inside `folder`.
    pub(crate) fn children(&self, folder: &str) -> BTreeSet<String> {
        let prefix = format!("{folder}/");
        self.paths
            .iter()
            .filter(|path| {
                path.strip_prefix(&prefix)
                    .is_some_and(|rest| !rest.contains('/'))
            })
            .cloned()
            .collect()
    }

    /// The top-level names (files and folders) the layout lists.
    pub(crate) fn top_level(&self) -> BTreeSet<String> {
        self.paths
            .iter()
            .map(|path| path.split('/').next().unwrap_or(path).to_owned())
            .collect()
    }

    /// The Rust workspace members the layout documents.
    pub(crate) fn rust_members(&self) -> BTreeSet<String> {
        let mut members = self.children("crates");
        if self.paths.iter().any(|path| path == "xtask") {
            members.insert("xtask".to_owned());
        }
        members
    }

    /// The browser packages the layout documents.
    pub(crate) fn npm_members(&self) -> BTreeSet<String> {
        let mut members = self.children("web/packages");
        if self.paths.iter().any(|path| path == "web/app") {
            members.insert("web/app".to_owned());
        }
        members
    }
}

/// Crate name -> the crates it may depend on.
pub(crate) type Map = BTreeMap<String, BTreeSet<String>>;

/// Reads the first Mermaid flowchart that follows the "Module map" heading.
///
/// `a --> b` and `a -.-> b` mean "a may depend on b"; `a --> b & c` lists several targets.
pub(crate) fn parse_module_map(markdown: &str) -> anyhow::Result<Map> {
    let after = markdown
        .split_once("## Module map")
        .map(|(_, rest)| rest)
        .with_context(|| format!("{BUILDING_BLOCKS} has no \"Module map\" heading"))?;
    let block = after
        .split_once("```mermaid")
        .and_then(|(_, rest)| rest.split_once("```"))
        .map(|(block, _)| block)
        .with_context(|| {
            format!("{BUILDING_BLOCKS} has no Mermaid flowchart under \"Module map\"")
        })?;

    let mut names = BTreeMap::new(); // box id -> crate name
    let mut edges = Vec::new();
    for line in block.lines().map(str::trim) {
        if let Some((id, rest)) = line.split_once("[\"") {
            let label = rest.split("\"]").next().unwrap_or("");
            let name = label.split("<br/>").next().unwrap_or(label);
            names.insert(id.trim().to_owned(), name.to_owned());
        } else if let Some((left, right)) = split_edge(line) {
            edges.push((left.to_owned(), right.to_owned()));
        }
    }
    let mut map: Map = names
        .values()
        .map(|name| (name.clone(), BTreeSet::new()))
        .collect();
    for (left, right) in &edges {
        for from in left.split('&').map(str::trim) {
            for to in right.split('&').map(str::trim) {
                let (Some(from), Some(to)) = (names.get(from), names.get(to)) else {
                    bail!(
                        "{BUILDING_BLOCKS}: the edge `{left} -> {right}` uses a box that is not declared"
                    );
                };
                map.entry(from.clone()).or_default().insert(to.clone());
            }
        }
    }
    Ok(map)
}

/// Splits `a --> b`, `a -.-> b` and `a -. text .-> b` into the two sides.
fn split_edge(line: &str) -> Option<(&str, &str)> {
    if let Some(at) = line.find("-->") {
        return Some((&line[..at], &line[at + 3..]));
    }
    let at = line.find("-.")?;
    let end = line[at + 1..].find(".->")? + at + 1;
    Some((&line[..at], &line[end + 3..]))
}

#[cfg(test)]
pub(crate) mod fixtures {
    //! A tiny layout for the tests of the checks that read the building blocks.

    use std::collections::BTreeSet;

    pub(crate) const LAYOUT: &str = "\
# Building blocks

## Repository layout

```text
bugaboo/
├── Cargo.toml              Rust workspace
├── crates/                 server code
│   ├── bb-core/            domain types
│   └── bb-api/             HTTP
├── xtask/                  runner
├── design/tokens.json      tokens
└── web/                    browser app
    ├── app/                the app
    └── packages/
        └── ui/             components
```

## Module map

```mermaid
flowchart TB
  core[\"bb-core\"]
  api[\"bb-api\"]
```
";

    pub(crate) fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }
}
