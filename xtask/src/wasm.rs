//! `cargo xtask wasm`: the WebAssembly module of the browser (ADR-0011, PERF-04).
//!
//! Three steps: cargo builds `bb-wasm` small (the `wasm` profile), `wasm-bindgen` writes the
//! module and its JavaScript glue, `wasm-opt` makes the module smaller. The result is in
//! `web/packages/wasm/generated/` (never committed). The folder remembers the hash of the module
//! cargo made, so a build without changes takes no time. ARCH-WASM holds the tools to the version
//! of the library, PERF-BUNDLE the size to its budget.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context as _, bail};
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use crate::{output, tools};

/// The crate that is built for the browser.
pub(crate) const CRATE: &str = "bb-wasm";
/// The name of the library, and of the program that joins the module with its glue: the same name,
/// and the same version (ARCH-WASM holds the two together).
pub(crate) const BINDGEN: &str = "wasm-bindgen";
/// The cargo profile with the settings for a small module (root `Cargo.toml`).
const PROFILE: &str = "wasm";
/// What the browser runs.
pub(crate) const TARGET: &str = "wasm32-unknown-unknown";
/// Where the module and its glue are written, relative to the repository root.
pub(crate) const OUT_DIR: &str = "web/packages/wasm/generated";
/// The name cargo gives the library of the crate, and the name of the files
/// (`<name>.js`, `<name>_bg.wasm`).
const OUT_NAME: &str = "bb_wasm";
/// Remembers which module cargo made last (its SHA-256), inside [`OUT_DIR`].
const STAMP: &str = ".stamp";
/// What `wasm-opt` is asked to do: optimize for size, with the features the compiler uses.
const OPTIMIZE: [&str; 5] = [
    "-Oz",
    "--enable-bulk-memory",
    "--enable-nontrapping-float-to-int",
    "--enable-sign-ext",
    "--enable-mutable-globals",
];

/// The folder of the module and its glue, with the separators of this system (a link made by
/// Windows needs them).
pub(crate) fn out_dir(root: &Path) -> PathBuf {
    OUT_DIR
        .split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part))
}

/// The module `wasm-opt` wrote.
pub(crate) fn module_path(root: &Path) -> PathBuf {
    out_dir(root).join(format!("{OUT_NAME}_bg.wasm"))
}

/// Builds the module, or says it is up to date. Returns whether it worked.
pub(crate) fn run(root: &Path) -> anyhow::Result<bool> {
    let built = cargo_build(root)?;
    let hash = hex(&Sha256::digest(
        std::fs::read(&built).with_context(|| format!("could not read {}", built.display()))?,
    ));
    let out = out_dir(root);
    std::fs::create_dir_all(&out).with_context(|| format!("could not make {}", out.display()))?;
    std::fs::write(out.join(BUILT), built.to_string_lossy().as_bytes())
        .context("could not write where cargo put the module")?;
    let stamp = out.join(STAMP);
    let remembered = std::fs::read_to_string(&stamp).unwrap_or_default();
    if remembered == hash && module_path(root).is_file() {
        // Written again, so the stamp is newer than the sources and `is_fresh` can say so.
        std::fs::write(&stamp, hash)
            .with_context(|| format!("could not write {}", stamp.display()))?;
        output::info("ok       WebAssembly module is up to date");
        return Ok(true);
    }
    std::fs::create_dir_all(&out).with_context(|| format!("could not make {}", out.display()))?;
    let bindgen = tools::find(BINDGEN).map_err(|finding| anyhow::anyhow!(finding.message))?;
    let built_text = built.to_string_lossy();
    run_tool(
        root,
        &bindgen,
        &[
            &built_text,
            "--target",
            "web",
            "--out-dir",
            OUT_DIR,
            "--out-name",
            OUT_NAME,
        ],
    )?;
    let optimizer = tools::find("wasm-opt").map_err(|finding| anyhow::anyhow!(finding.message))?;
    let module = module_path(root);
    let module_text = module.to_string_lossy();
    let mut args: Vec<&str> = vec![&module_text];
    args.extend(OPTIMIZE);
    args.extend(["-o", &module_text]);
    run_tool(root, &optimizer, &args)?;
    std::fs::write(&stamp, hash).with_context(|| format!("could not write {}", stamp.display()))?;
    output::info(&format!("wrote    {OUT_DIR}/{OUT_NAME}_bg.wasm"));
    Ok(true)
}

/// Remembers where cargo put the module, inside [`OUT_DIR`]: cargo's own list of what the module
/// was made from (its dep-info files) is found from there.
const BUILT: &str = ".built";
/// Files outside the Rust sources that change the module: the toolchain, the settings of cargo,
/// the pinned tools. The version of the program that builds it (`xtask`) counts too.
const OTHER_INPUTS: [&str; 5] = [
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo",
    "tools/dev/tools.toml",
];

/// The newest change in a file or a folder (below it), if any.
fn newest(path: &Path) -> Option<std::time::SystemTime> {
    let meta = std::fs::metadata(path).ok()?;
    let own = meta.modified().ok();
    if !meta.is_dir() {
        return own;
    }
    std::fs::read_dir(path)
        .ok()?
        .flatten()
        .filter_map(|entry| newest(&entry.path()))
        .chain(own)
        .max()
}

/// The files of this repository that cargo says the module was made from: the source files in the
/// dep-info (`.d`) files next to it, and the `Cargo.toml` of each crate they belong to.
fn built_from(root: &Path, built: &Path) -> Vec<PathBuf> {
    let Ok(entries) = built
        .parent()
        .map_or_else(|| PathBuf::from("deps"), |dir| dir.join("deps"))
        .read_dir()
    else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for entry in entries.flatten().map(|entry| entry.path()) {
        if entry.extension().is_none_or(|extension| extension != "d") {
            continue;
        }
        let text = std::fs::read_to_string(&entry).unwrap_or_default();
        // `output: input input …` on the first line; the lines after it are empty rules.
        let inputs = text
            .lines()
            .next()
            .and_then(|line| line.split_once(": "))
            .map_or("", |(_, inputs)| inputs);
        for input in inputs.split_whitespace() {
            let path = root.join(input);
            if path.starts_with(root) {
                let manifest = path
                    .ancestors()
                    .skip(1)
                    .take_while(|folder| folder.starts_with(root))
                    .map(|folder| folder.join("Cargo.toml"))
                    .find(|manifest| manifest.is_file());
                files.extend(manifest);
                files.push(path);
            }
        }
    }
    files
}

/// Whether the module is newer than everything it is made from. The checks ask this first, because
/// asking cargo takes longer than the check can spare. What cargo listed is read from its own
/// files; the rest is [`OTHER_INPUTS`] and the program that builds.
fn is_fresh(root: &Path) -> bool {
    let out = out_dir(root);
    let Some(stamped) = newest(&out.join(STAMP)) else {
        return false;
    };
    let Ok(built) = std::fs::read_to_string(out.join(BUILT)) else {
        return false;
    };
    let others = OTHER_INPUTS.iter().map(|input| root.join(input));
    let program = std::env::current_exe().ok();
    built_from(root, Path::new(built.trim()))
        .into_iter()
        .chain(others)
        .chain(program)
        .all(|input| newest(&input).is_none_or(|changed| changed <= stamped))
        && module_path(root).is_file()
}

/// Makes sure the module is there; for checks and tests that read it.
pub(crate) fn ensure(root: &Path) -> anyhow::Result<()> {
    if is_fresh(root) {
        return Ok(());
    }
    if run(root)? {
        Ok(())
    } else {
        bail!("the WebAssembly module could not be built (see the messages above)")
    }
}

fn run_tool(root: &Path, program: &Path, args: &[&str]) -> anyhow::Result<()> {
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .status()
        .with_context(|| format!("could not start {}", program.display()))?;
    if status.success() {
        Ok(())
    } else {
        bail!("{} failed (see the messages above)", program.display())
    }
}

/// Runs cargo and returns the module it wrote. Cargo says where: a different `target-dir` in the
/// settings of the machine needs no guessing.
fn cargo_build(root: &Path) -> anyhow::Result<PathBuf> {
    let out = Command::new("cargo")
        .args([
            "build",
            "--message-format=json-render-diagnostics",
            "--package",
            CRATE,
            "--target",
            TARGET,
            "--profile",
            PROFILE,
        ])
        .current_dir(root)
        .stderr(Stdio::inherit())
        .output()
        .context("could not run cargo")?;
    if !out.status.success() {
        bail!("cargo could not build {CRATE} for {TARGET} (see the messages above)");
    }
    module_from(&String::from_utf8_lossy(&out.stdout))
        .with_context(|| format!("cargo did not report a module for {CRATE}"))
}

/// The `.wasm` file of the crate in cargo's JSON messages.
fn module_from(cargo_output: &str) -> Option<PathBuf> {
    cargo_output
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|message| message["reason"] == "compiler-artifact")
        .filter(|message| message["target"]["name"] == OUT_NAME)
        .filter_map(|message| {
            message["filenames"]
                .as_array()?
                .iter()
                .filter_map(Value::as_str)
                .find(|name| {
                    Path::new(name)
                        .extension()
                        .is_some_and(|extension| extension == "wasm")
                })
                .map(PathBuf::from)
        })
        .next_back()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut text, byte| {
        // Writing into a String cannot fail.
        let _ = write!(text, "{byte:02x}");
        text
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_module_comes_from_the_compiler_artifact_message() {
        let output = [
            r#"{"reason":"compiler-artifact","target":{"name":"bb_markdown"},"filenames":["a/libbb_markdown.rlib"]}"#,
            r#"{"reason":"compiler-artifact","target":{"name":"bb_wasm"},"filenames":["a/libbb_wasm.rlib","D:/custom target/wasm/bb_wasm.wasm"]}"#,
            "not json",
        ]
        .join("\n");
        assert_eq!(
            module_from(&output),
            Some(PathBuf::from("D:/custom target/wasm/bb_wasm.wasm"))
        );
        assert_eq!(module_from(""), None);
    }

    #[test]
    fn the_build_is_fresh_until_something_it_was_made_from_changes() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path();
        assert!(!is_fresh(path), "nothing built yet");
        // Cargo's dep-info says what the module was made from.
        let target = path.join("target").join("wasm");
        std::fs::create_dir_all(target.join("deps")).unwrap();
        let source = path.join("crates").join("a").join("lib.rs");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, b"fn a() {}").unwrap();
        let info = format!(
            "x.wasm: {}

{}:
",
            source.display(),
            source.display()
        );
        std::fs::write(target.join("deps").join("a.d"), info).unwrap();
        std::fs::create_dir_all(out_dir(path)).unwrap();
        std::fs::write(module_path(path), b"module").unwrap();
        let built = target.join("a.wasm");
        std::fs::write(
            out_dir(path).join(BUILT),
            built.to_string_lossy().as_bytes(),
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(out_dir(path).join(STAMP), b"hash").unwrap();
        // The program that runs this test was built before the stamp was written.
        assert!(is_fresh(path));
        assert_eq!(built_from(path, &built), [source.clone()]);
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(&source, b"fn b() {}").unwrap();
        assert!(!is_fresh(path), "the source is newer than the stamp");
    }

    #[test]
    fn a_hash_is_written_in_hex() {
        assert_eq!(hex(&[0, 15, 255]), "000fff");
    }
}
