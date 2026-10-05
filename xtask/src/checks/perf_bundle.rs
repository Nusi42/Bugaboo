//! PERF-BUNDLE: the JavaScript the first page loads stays under its budget (PERF-04, ADR-0009).
//!
//! Reads the Vite manifest of the built browser app, adds up the JavaScript the first route
//! loads (every entry and, from there, the files it imports at once, never the ones it loads
//! later with `import()`), compresses each file with Brotli at the quality the server uses
//! (`tools/dev/compression.json`, read by the server too) and fails above the budget. Heavy features belong behind a lazy `import()` (coding-standards §6).
//!
//! The budget and the places to read are the settings in `tools/checks/perf-bundle.toml`, and
//! nowhere else. A fixture folder holds its own settings file with a small budget, so the
//! examples stay small. WebAssembly joins the budget when `bb-wasm` exists (M1-F10).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Context as _;
use serde::Deserialize;
use serde_json::Value;

use super::{Check, Context, Finding};

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
        evaluate(root, &settings, compression_quality(root)?)
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

    fn repository() -> std::path::PathBuf {
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
    fn the_budget_is_read_from_the_settings_file() {
        let settings = Settings::load(&repository().join(SETTINGS_PATH)).unwrap();
        assert!(settings.budget_bytes > 0);
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
