//! PERF-BUNDLE: the JavaScript the first page loads stays under its budget (PERF-04, ADR-0009).
//!
//! Reads the Vite manifest of the built browser app, adds up the JavaScript the first route
//! loads (every entry and, from there, the files it imports at once, never the ones it loads
//! later with `import()`), compresses each file with Brotli at the quality the server uses
//! (`tools/dev/compression.json`, read by the server too) and fails above the budget. Heavy features belong behind a lazy `import()` (coding-standards §6).
//!
//! The budget and the places to read are the settings in `tools/checks/perf-bundle.toml`, and
//! nowhere else. A fixture folder holds its own settings file with a small budget, so the
//! examples stay small. Every WebAssembly module has a budget of its own, found by the name of the
//! module (`out_name` of `MODULES`, then `_bg`) that its file starts with: `bb_wasm_search_bg-3f9-a1c.wasm`
//! is `bb_wasm_search_bg`, whatever characters the hash holds. A list should not fetch the module of
//! Markdown, nor an editor the search language (ADR-0005, amendment M5). A module of `MODULES`
//! without a line in the settings is a finding, so a new module cannot slip in, and so is a line
//! that names no module, so a budget cannot outlive its module.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use serde::Deserialize;
use serde_json::Value;

use super::{Check, Context, Finding};
use crate::wasm::{MODULES, Module};

/// Where the settings are, relative to the repository root.
const SETTINGS_PATH: &str = "tools/checks/perf-bundle.toml";
/// The compression settings the server reads too (`crates/bb-api/src/router.rs`): the quality
/// is defined there once, so the size measured here is the size a browser receives.
const COMPRESSION_PATH: &str = "tools/dev/compression.json";
/// How many of the biggest files a finding names.
const BIGGEST_SHOWN: usize = 3;

pub(crate) struct PerfBundle;

impl Check for PerfBundle {
    fn id(&self) -> &'static str {
        "PERF-BUNDLE"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let root = context.repo.root();
        let settings = Settings::load(&root.join(SETTINGS_PATH))?;
        let mut findings = list_findings(&settings);
        findings.extend(evaluate(root, &settings, compression_quality(root)?)?);
        Ok(findings)
    }
}

/// The settings file.
#[derive(Deserialize)]
struct Settings {
    /// The folder the build wrote, relative to the root the check reads from.
    dist: String,
    /// The manifest, relative to `dist`.
    manifest: String,
    /// The most Brotli bytes the first page may load (1 KB is 1000 bytes).
    budget_bytes: u64,
    /// The most Brotli bytes each WebAssembly module of the build may have, by the name of its file
    /// without the hash and the extension.
    wasm_budget_bytes: BTreeMap<String, u64>,
}

impl Settings {
    fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("could not read {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("{} is not valid", path.display()))
    }
}

/// The quality the server compresses with.
fn compression_quality(root: &Path) -> anyhow::Result<i32> {
    let path = root.join(COMPRESSION_PATH);
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("could not read {}", path.display()))?;
    let settings: Value = serde_json::from_str(&text)
        .with_context(|| format!("{} is not valid JSON", path.display()))?;
    settings
        .get("quality")
        .and_then(Value::as_i64)
        .and_then(|quality| i32::try_from(quality).ok())
        .with_context(|| format!("{} has no `quality` number", path.display()))
}

/// The JavaScript files of the first page: every entry of the manifest and what it imports at
/// once, each file once, in the order they are found.
fn first_route_files(manifest: &Value) -> anyhow::Result<Vec<String>> {
    let chunks = manifest
        .as_object()
        .context("the manifest is not a JSON object")?;
    let mut files = Vec::new();
    let mut seen = BTreeSet::new();
    let mut pending: Vec<&str> = chunks
        .iter()
        .filter(|(_, chunk)| chunk.get("isEntry").and_then(Value::as_bool) == Some(true))
        .map(|(key, _)| key.as_str())
        .collect();
    pending.reverse();
    while let Some(key) = pending.pop() {
        if !seen.insert(key) {
            continue;
        }
        let chunk = chunks
            .get(key)
            .with_context(|| format!("the manifest names `{key}` but has no entry for it"))?;
        if let Some(file) = chunk.get("file").and_then(Value::as_str)
            && has_extension(file, &["js", "mjs"])
        {
            files.push(file.to_owned());
        }
        let imports = chunk
            .get("imports")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str);
        let mut next: Vec<&str> = imports.collect();
        next.reverse();
        pending.extend(next);
    }
    Ok(files)
}

fn has_extension(file: &str, extensions: &[&str]) -> bool {
    Path::new(file)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extensions.contains(&extension))
}

/// How many bytes the file has after Brotli at `quality`, with the window the server uses (the
/// default of the library).
fn brotli_size(bytes: &[u8], quality: i32) -> anyhow::Result<u64> {
    let parameters = brotli::enc::BrotliEncoderParams {
        quality,
        ..Default::default()
    };
    let mut input = bytes;
    let mut output = Vec::new();
    brotli::BrotliCompress(&mut input, &mut output, &parameters)
        .context("could not compress a file with Brotli")?;
    u64::try_from(output.len()).context("a compressed file is too big to count")
}

/// A size in kilobytes with one decimal (1 KB is 1000 bytes), made with whole numbers.
fn kilobytes(bytes: u64) -> String {
    format!("{}.{} KB", bytes / 1000, bytes % 1000 / 100)
}

fn evaluate(root: &Path, settings: &Settings, quality: i32) -> anyhow::Result<Vec<Finding>> {
    let mut findings = script_findings(root, settings, quality)?;
    findings.extend(wasm_findings(root, settings, quality)?);
    Ok(findings)
}

fn script_findings(root: &Path, settings: &Settings, quality: i32) -> anyhow::Result<Vec<Finding>> {
    let dist = root.join(&settings.dist);
    let manifest_path = dist.join(&settings.manifest);
    let Ok(text) = std::fs::read_to_string(&manifest_path) else {
        return Ok(vec![Finding::at(
            SETTINGS_PATH,
            format!(
                "There is no manifest at {}. Build the browser app first (`npm run build`); the budget is read from the build.",
                manifest_path.display()
            ),
        )]);
    };
    let manifest: Value = serde_json::from_str(&text)
        .with_context(|| format!("{} is not valid JSON", manifest_path.display()))?;
    let files = first_route_files(&manifest)?;
    if files.is_empty() {
        return Ok(vec![Finding::at(
            manifest_path.display().to_string(),
            "The manifest has no entry with JavaScript, so there is nothing to measure.",
        )]);
    }
    let mut sizes = BTreeMap::new();
    for file in &files {
        let path = dist.join(file);
        let bytes = std::fs::read(&path).with_context(|| {
            format!("could not read {} (named in the manifest)", path.display())
        })?;
        sizes.insert(file.clone(), brotli_size(&bytes, quality)?);
    }
    let total: u64 = sizes.values().sum();
    if total <= settings.budget_bytes {
        return Ok(Vec::new());
    }
    let mut biggest: Vec<(&String, &u64)> = sizes.iter().collect();
    biggest.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    let named: Vec<String> = biggest
        .iter()
        .take(BIGGEST_SHOWN)
        .map(|(file, size)| format!("{file} ({})", kilobytes(**size)))
        .collect();
    Ok(vec![Finding::at(
        manifest_path.display().to_string(),
        format!(
            "The first page loads {} of JavaScript with Brotli; the budget is {} (PERF-04). Biggest: {}. Load heavy features with a lazy `import()` so they are fetched when first opened.",
            kilobytes(total),
            kilobytes(settings.budget_bytes),
            named.join(", ")
        ),
    )])
}

/// Every `.wasm` file below `folder`, in the order of their names.
fn wasm_files(folder: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    let mut pending = vec![folder.to_path_buf()];
    while let Some(next) = pending.pop() {
        let entries = std::fs::read_dir(&next)
            .with_context(|| format!("could not read the folder {}", next.display()))?;
        for entry in entries {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if has_extension(&path.to_string_lossy(), &["wasm"]) {
                found.push(path);
            }
        }
    }
    found.sort();
    Ok(found)
}

/// The name a module of `MODULES` has in the settings: the name of its files without the hash.
fn settings_name(module: &Module) -> String {
    format!("{}_bg", module.out_name)
}

/// The names a file may have in the settings: those of the modules of `MODULES`, and the lines the
/// settings have (the examples of this check hold modules of their own).
fn known_names(settings: &Settings) -> BTreeSet<String> {
    MODULES
        .iter()
        .map(settings_name)
        .chain(settings.wasm_budget_bytes.keys().cloned())
        .collect()
}

/// The name of a module in the settings: the known name that its file starts with, followed by the
/// end of the name or by the dash before the hash the build adds. The hash is never cut at a dash
/// of its own, because the dashes in it are not looked at. A file that no known name fits is named
/// by its whole name, which then has no budget.
fn module_name(path: &Path, known: &BTreeSet<String>) -> String {
    let stem = path
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    known
        .iter()
        .filter(|name| {
            stem == **name
                || stem
                    .strip_prefix(name.as_str())
                    .is_some_and(|rest| rest.starts_with('-'))
        })
        .max_by_key(|name| name.len())
        .cloned()
        .unwrap_or(stem)
}

/// The WebAssembly modules to measure: the files in the build of the browser app and, for a module
/// that no page imports yet, the file `cargo xtask wasm` made, so a module is held to its budget
/// from the day it exists.
fn wasm_modules(
    root: &Path,
    settings: &Settings,
    known: &BTreeSet<String>,
) -> anyhow::Result<Vec<PathBuf>> {
    let dist = root.join(&settings.dist);
    let mut found = if dist.is_dir() {
        wasm_files(&dist)?
    } else {
        Vec::new()
    };
    for module in MODULES {
        let made = module.path(root);
        let measured = found
            .iter()
            .any(|path| module_name(path, known) == module_name(&made, known));
        if !measured && made.is_file() {
            found.push(made);
        }
    }
    Ok(found)
}

/// Every module of `MODULES` has a line in the settings, and the settings have no other line than
/// the lines of the modules that are built (told by `wasm_findings`): a new module cannot slip in
/// without a budget.
fn list_findings(settings: &Settings) -> Vec<Finding> {
    let mut findings = Vec::new();
    for module in &MODULES {
        let name = settings_name(module);
        if !settings.wasm_budget_bytes.contains_key(&name) {
            findings.push(Finding::at(
                SETTINGS_PATH,
                format!(
                    "The WebAssembly module {} (`{}` in `MODULES`) has no budget line: add `{name} = <bytes>` to [wasm_budget_bytes] in {SETTINGS_PATH}, from its measured size.",
                    module.out_name, module.crate_name
                ),
            ));
        }
    }
    findings
}

/// Each WebAssembly module stays within the budget it has in the settings. A module is fetched when
/// its feature first opens, not with the first page, but every browser that opens the feature
/// receives it (PERF-04).
fn wasm_findings(root: &Path, settings: &Settings, quality: i32) -> anyhow::Result<Vec<Finding>> {
    let known = known_names(settings);
    let mut findings = Vec::new();
    let files = wasm_modules(root, settings, &known)?;
    let measured: BTreeSet<String> = files.iter().map(|path| module_name(path, &known)).collect();
    for name in settings.wasm_budget_bytes.keys() {
        let is_a_module = MODULES.iter().any(|module| settings_name(module) == *name);
        if !is_a_module && !measured.contains(name) {
            findings.push(Finding::at(
                SETTINGS_PATH,
                format!(
                    "The line `{name}` in [wasm_budget_bytes] names no WebAssembly module: take it out, or add the module to `MODULES` in xtask/src/wasm.rs."
                ),
            ));
        }
    }
    for path in files {
        let bytes =
            std::fs::read(&path).with_context(|| format!("could not read {}", path.display()))?;
        let size = brotli_size(&bytes, quality)?;
        let file = path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let name = module_name(&path, &known);
        match settings.wasm_budget_bytes.get(&name) {
            // A module of `MODULES` without a line is told above, once.
            None if MODULES.iter().any(|module| settings_name(module) == name) => {}
            None => findings.push(Finding::at(
                SETTINGS_PATH,
                format!(
                    "The WebAssembly module {file} has no budget: add a line `{name} = <bytes>` to [wasm_budget_bytes] in {SETTINGS_PATH}, from its measured size ({} with Brotli).",
                    kilobytes(size)
                ),
            )),
            Some(budget) if size > *budget => findings.push(Finding::at(
                SETTINGS_PATH,
                format!(
                    "The WebAssembly module {file} is {} with Brotli; its budget is {} (PERF-04). Take out what the browser does not need, or the module grows past what a phone can fetch quickly.",
                    kilobytes(size),
                    kilobytes(*budget)
                ),
            )),
            Some(_) => {}
        }
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// A fixture folder: `tools/checks/fixtures/PERF-BUNDLE/<kind>`, with its own settings.
    fn run_fixture(kind: &str) -> Vec<Finding> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tools/checks/fixtures/PERF-BUNDLE")
            .join(kind);
        let settings = Settings::load(&root.join("perf-bundle.toml")).unwrap();
        evaluate(&root, &settings, server_quality()).unwrap()
    }

    fn repository() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    fn server_quality() -> i32 {
        compression_quality(&repository()).unwrap()
    }

    #[test]
    fn bad_example() {
        let found = run_fixture("bad");
        assert_eq!(found.len(), 1);
        let message = &found[0].message;
        assert!(message.contains("budget is 2.0 KB"), "{message}");
        assert!(message.contains("assets/vendor-"), "{message}");
    }

    #[test]
    fn good_example() {
        assert!(run_fixture("good").is_empty());
    }

    #[test]
    fn bad_example_a_module_over_its_budget() {
        let found = run_fixture("bad-wasm");
        assert_eq!(found.len(), 1);
        let message = &found[0].message;
        assert!(
            message.contains("WebAssembly module module-dddd.wasm"),
            "{message}"
        );
        assert!(message.contains("its budget is 0.1 KB"), "{message}");
    }

    #[test]
    fn bad_example_a_module_without_a_budget() {
        let root = repository().join("tools/checks/fixtures/PERF-BUNDLE/good");
        let mut settings = Settings::load(&root.join("perf-bundle.toml")).unwrap();
        settings.wasm_budget_bytes.clear();
        let found = evaluate(&root, &settings, server_quality()).unwrap();
        assert_eq!(found.len(), 1);
        assert!(
            found[0].message.contains("has no budget"),
            "{}",
            found[0].message
        );
        assert!(
            found[0].message.contains("`module-dddd = <bytes>`"),
            "{}",
            found[0].message
        );
    }

    #[test]
    fn a_module_is_known_by_its_name_without_the_hash() {
        let known: BTreeSet<String> = ["bb_wasm_bg", "bb_wasm_search_bg", "module"]
            .map(str::to_owned)
            .into();
        let name = |file: &str| module_name(Path::new(file), &known);
        assert_eq!(
            name("assets/bb_wasm_search_bg-3f9a1c.wasm"),
            "bb_wasm_search_bg"
        );
        // A hash can hold dashes (Vite writes base64url): the name is not cut at the last one.
        assert_eq!(
            name("assets/bb_wasm_search_bg-Q3-f_9a-1c.wasm"),
            "bb_wasm_search_bg"
        );
        assert_eq!(name("assets/bb_wasm_bg-aB-cD.wasm"), "bb_wasm_bg");
        assert_eq!(name("bb_wasm_search_bg.wasm"), "bb_wasm_search_bg");
        assert_eq!(name("module-dddd.wasm"), "module");
        // A name that only starts like a known one is another module.
        assert_eq!(name("bb_wasm_bgx-1.wasm"), "bb_wasm_bgx-1");
        assert_eq!(name("plain.wasm"), "plain");
    }

    #[test]
    fn bad_example_a_module_of_the_list_without_a_budget_line() {
        let mut settings = Settings::load(&repository().join(SETTINGS_PATH)).unwrap();
        assert!(list_findings(&settings).is_empty());
        let first = settings_name(&MODULES[0]);
        settings.wasm_budget_bytes.remove(&first);
        let found = list_findings(&settings);
        assert_eq!(found.len(), 1);
        assert!(
            found[0]
                .message
                .contains(&format!("add `{first} = <bytes>`")),
            "{}",
            found[0].message
        );
    }

    #[test]
    fn bad_example_a_budget_line_that_names_no_module() {
        let root = repository().join("tools/checks/fixtures/PERF-BUNDLE/good");
        let mut settings = Settings::load(&root.join("perf-bundle.toml")).unwrap();
        settings.wasm_budget_bytes.insert("gone".to_owned(), 1);
        let found = wasm_findings(&root, &settings, server_quality()).unwrap();
        assert_eq!(
            found.len(),
            1,
            "{:?}",
            found
                .iter()
                .map(|finding| &finding.message)
                .collect::<Vec<_>>()
        );
        assert!(
            found[0].message.contains("The line `gone`"),
            "{:?}",
            found
                .iter()
                .map(|finding| &finding.message)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn this_repository_has_a_budget_line_for_each_module_and_no_other() {
        let settings = Settings::load(&repository().join(SETTINGS_PATH)).unwrap();
        let lines: BTreeSet<&String> = settings.wasm_budget_bytes.keys().collect();
        let modules: Vec<String> = MODULES.iter().map(settings_name).collect();
        assert_eq!(lines, modules.iter().collect::<BTreeSet<_>>());
    }

    #[test]
    fn the_budget_is_read_from_the_settings_file() {
        let settings = Settings::load(&repository().join(SETTINGS_PATH)).unwrap();
        assert!(settings.budget_bytes > 0);
        assert!(!settings.wasm_budget_bytes.is_empty());
        assert!(
            settings
                .wasm_budget_bytes
                .values()
                .all(|budget| *budget > 0)
        );
    }

    /// The quality is the server's: the server includes the very file this check reads, and sets
    /// no quality of its own anywhere else.
    #[test]
    fn the_budget_is_measured_with_the_quality_the_server_compresses_with() {
        let router =
            std::fs::read_to_string(repository().join("crates/bb-api/src/router.rs")).unwrap();
        assert!(
            router.contains(&format!("include_str!(\"../../../{COMPRESSION_PATH}\")")),
            "the server must read its compression quality from {COMPRESSION_PATH}"
        );
        assert!(
            !router.contains("Precise("),
            "the server must take its quality from the file, not from a number in the code"
        );
        assert!(server_quality() > 0);
    }

    #[test]
    fn the_first_page_is_the_entries_and_what_they_import_at_once() {
        let manifest = json!({
            "index.html": {"file": "assets/index-1.js", "isEntry": true, "imports": ["_vendor.js", "_shared.js"], "dynamicImports": ["lazy.ts"], "css": ["assets/index-1.css"]},
            "_vendor.js": {"file": "assets/vendor-2.js", "imports": ["_shared.js"]},
            "_shared.js": {"file": "assets/shared-3.js"},
            "lazy.ts": {"file": "assets/lazy-4.js", "isDynamicEntry": true},
        });
        assert_eq!(
            first_route_files(&manifest).unwrap(),
            [
                "assets/index-1.js",
                "assets/vendor-2.js",
                "assets/shared-3.js"
            ]
        );
    }

    #[test]
    fn only_javascript_is_counted() {
        let manifest = json!({"index.html": {"file": "assets/index-1.css", "isEntry": true}});
        assert!(first_route_files(&manifest).unwrap().is_empty());
    }

    #[test]
    fn a_missing_manifest_says_to_build_first() {
        let root = tempfile::tempdir().unwrap();
        let settings = Settings {
            dist: "dist".to_owned(),
            manifest: "manifest.json".to_owned(),
            budget_bytes: 1,
            wasm_budget_bytes: BTreeMap::new(),
        };
        let found = evaluate(root.path(), &settings, 4).unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("Build the browser app first"));
    }

    #[test]
    fn a_manifest_that_names_a_missing_chunk_is_an_error() {
        let manifest =
            json!({"index.html": {"file": "a.js", "isEntry": true, "imports": ["nope"]}});
        assert!(first_route_files(&manifest).is_err());
    }
}
