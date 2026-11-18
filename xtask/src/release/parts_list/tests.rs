use std::collections::BTreeSet;

use serde_json::json;

use super::*;
use crate::release::{repository_root, this_repository};

/// A tiny `cargo metadata`: the program uses `serde` (normal), `cc` (build) and `trial` (test).
/// `serde` uses `serde_derive`; `cc` uses `shlex`.
const METADATA: &str = r#"{
  "packages": [
    {"id":"app","name":"app","version":"1.0.0","license":"AGPL-3.0-or-later","source":null,"manifest_path":"/w/app/Cargo.toml"},
    {"id":"serde","name":"serde","version":"1.0.2","license":"MIT/Apache-2.0","source":"registry","manifest_path":"/r/serde/Cargo.toml"},
    {"id":"serde_derive","name":"serde_derive","version":"1.0.2","license":"MIT","source":"registry","manifest_path":"/r/serde_derive/Cargo.toml"},
    {"id":"cc","name":"cc","version":"1.2.0","license":"MIT","source":"registry","manifest_path":"/r/cc/Cargo.toml"},
    {"id":"shlex","name":"shlex","version":"1.3.0","license":"MIT","source":"registry","manifest_path":"/r/shlex/Cargo.toml"},
    {"id":"trial","name":"trial","version":"0.1.0","license":"MIT","source":"registry","manifest_path":"/r/trial/Cargo.toml"}
  ],
  "resolve": {"nodes": [
    {"id":"app","deps":[
      {"pkg":"serde","dep_kinds":[{"kind":null}]},
      {"pkg":"cc","dep_kinds":[{"kind":"build"}]},
      {"pkg":"trial","dep_kinds":[{"kind":"dev"}]}]},
    {"id":"serde","deps":[{"pkg":"serde_derive","dep_kinds":[{"kind":null}]}]},
    {"id":"serde_derive","deps":[]},
    {"id":"cc","deps":[{"pkg":"shlex","dep_kinds":[{"kind":null}]}]},
    {"id":"shlex","deps":[]},
    {"id":"trial","deps":[]}
  ]}
}"#;

fn names(closure: &Closure) -> Vec<String> {
    closure
        .crates
        .keys()
        .map(|(name, _)| name.clone())
        .collect()
}

#[test]
fn only_normal_dependencies_are_compiled_in() {
    let closure = rust_closure(METADATA, &["app"], None).unwrap();
    assert_eq!(names(&closure), ["serde", "serde_derive"]);
    assert_eq!(closure.program.unwrap().license, "AGPL-3.0-or-later");
    let direct: Vec<_> = closure
        .direct
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(direct, ["serde"]);
}

#[test]
fn a_slash_in_a_cargo_license_becomes_or() {
    let closure = rust_closure(METADATA, &["app"], None).unwrap();
    let serde = &closure.crates[&("serde".to_owned(), "1.0.2".to_owned())];
    assert_eq!(serde.license.as_deref(), Some("MIT OR Apache-2.0"));
    assert!(
        serde
            .depends_on
            .contains(&("serde_derive".to_owned(), "1.0.2".to_owned()))
    );
}

#[test]
fn a_root_that_is_not_in_the_workspace_is_an_error() {
    assert!(rust_closure(METADATA, &["missing"], None).is_err());
}

#[test]
fn checksums_come_from_the_lockfile() {
    let sums = lock_checksums(
        "[[package]]\nname = \"serde\"\nversion = \"1.0.2\"\nchecksum = \"abc\"\n\n[[package]]\nname = \"app\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    assert_eq!(sums.len(), 1);
    assert_eq!(sums[&("serde".to_owned(), "1.0.2".to_owned())], "abc");
}

#[test]
fn base64_is_decoded() {
    assert_eq!(base64_decode("aGVsbG8=").unwrap(), b"hello");
    assert_eq!(base64_decode("aGVsbG8").unwrap(), b"hello");
    assert!(base64_decode("not base64!").is_none());
    let checksum = integrity_checksum("sha512-aGVsbG8=").unwrap();
    assert_eq!(checksum.algorithm, Algorithm::Sha512);
    assert_eq!(checksum.hex, "68656c6c6f");
}

/// An app that asks for `a` and for our own package `@bugaboo/ui`; `a` asks for `b`, which a
/// nested folder of `a` shadows; `tool` is only a dev tool; `peer` is an optional peer that is not
/// installed.
fn lock() -> Value {
    json!({"packages": {
        "": {},
        "web/app": {"dependencies": {"a": "1", "@bugaboo/ui": "*"}, "devDependencies": {"tool": "1"}},
        "web/ui": {"dependencies": {"u": "1"}},
        "node_modules/@bugaboo/ui": {"resolved": "web/ui", "link": true},
        "node_modules/a": {"version": "1.0.0", "license": "MIT", "integrity": "sha512-aGVsbG8=",
            "dependencies": {"b": "2"}, "peerDependencies": {"peer": "1"},
            "peerDependenciesMeta": {"peer": {"optional": true}}},
        "node_modules/b": {"version": "1.0.0", "license": "MIT", "integrity": "sha512-aGVsbG8="},
        "node_modules/a/node_modules/b": {"version": "2.0.0", "integrity": "sha512-aGVsbG8="},
        "node_modules/u": {"version": "3.0.0", "license": "ISC", "integrity": "sha512-aGVsbG8="},
        "node_modules/tool": {"version": "1.0.0", "dev": true, "license": "MIT"}
    }})
}

#[test]
fn the_browser_app_bundles_what_it_depends_on_and_not_its_tools() {
    let (parts, direct) = npm_closure(&lock(), &|key| {
        (key == "node_modules/a/node_modules/b").then(|| json!({"license": "BSD-3-Clause"}))
    })
    .unwrap();
    let listed: Vec<String> = parts
        .iter()
        .map(|part| format!("{}@{}", part.name, part.version))
        .collect();
    assert_eq!(listed, ["a@1.0.0", "b@2.0.0", "u@3.0.0"]);
    assert_eq!(direct.len(), 2, "a, and u through our own package");
    let b = &parts[1];
    assert_eq!(
        b.license.as_deref(),
        Some("BSD-3-Clause"),
        "from the installed package.json"
    );
    assert!(parts[0].depends_on.contains("pkg:npm/b@2.0.0"));
    assert!(parts.iter().all(|part| part.checksum.is_some()));
}

#[test]
fn a_crate_that_cargo_tree_does_not_name_is_left_out_with_what_is_below_it() {
    let only: BTreeSet<(String, String)> = [("serde".to_owned(), "1.0.2".to_owned())].into();
    let closure = rust_closure(METADATA, &["app"], Some(&only)).unwrap();
    assert_eq!(names(&closure), ["serde"]);
    assert!(
        closure
            .crates
            .values()
            .all(|found| found.depends_on.is_empty())
    );
}

#[test]
fn comp_01_the_parts_list_names_every_crate_the_program_compiles_in() {
    // `cargo tree` resolves the features of one program by itself; the list is held to it in both
    // directions: no crate it names is missing, and none is listed that it does not name.
    let listed: BTreeSet<(String, String)> = this_repository()
        .list
        .parts
        .iter()
        .filter(|part| part.kind == Kind::Crate)
        .map(|part| (part.name.clone(), part.version.clone()))
        .collect();
    let compiled = tree_union(&repository_root()).unwrap();
    assert!(compiled.len() > 300, "only {} crates", compiled.len());
    let missing: Vec<_> = compiled.difference(&listed).collect();
    let extra: Vec<_> = listed.difference(&compiled).collect();
    assert!(
        missing.is_empty(),
        "compiled in and not listed: {missing:?}"
    );
    assert!(extra.is_empty(), "listed and not compiled in: {extra:?}");
}

#[test]
fn comp_01_the_parts_list_names_every_npm_package_the_browser_app_bundles() {
    let root = repository_root();
    let lock: Value =
        serde_json::from_str(&fs::read_to_string(root.join("package-lock.json")).unwrap()).unwrap();
    let listed: BTreeSet<(String, String)> = this_repository()
        .list
        .parts
        .iter()
        .filter(|part| part.kind == Kind::Npm)
        .map(|part| (part.name.clone(), part.version.clone()))
        .collect();
    assert_eq!(
        listed,
        locked_not_dev(&lock),
        "the packages the lockfile does not mark dev (or dev and optional) are the ones the app bundles"
    );
}

#[test]
fn the_notice_files_name_their_license() {
    assert_eq!(
        notice_license("Fonts\nLicense: x\nSPDX-License-Identifier: CC0-1.0\n"),
        Some("CC0-1.0".to_owned())
    );
    assert_eq!(notice_license("Fonts\nLicense: x\n"), None);
    let notices = notice_files(&repository_root());
    assert!(notices.len() >= 2);
    for path in notices {
        let text = fs::read_to_string(repository_root().join(&path)).unwrap();
        assert!(
            notice_license(&text).is_some(),
            "{path} names no SPDX license"
        );
    }
}

#[test]
fn the_register_data_files_and_notices_are_parts() {
    let list = &this_repository().list;
    let files: Vec<&str> = list
        .parts
        .iter()
        .filter(|part| part.kind == Kind::File)
        .map(|part| part.name.as_str())
        .collect();
    assert!(files.contains(&"crates/bb-auth/data/breached-passwords.txt"));
    assert!(
        files
            .iter()
            .any(|name| name.ends_with("hayro-interpret-fonts-bsd-3-clause.txt"))
    );
    assert!(list.parts.iter().all(|part| part.checksum.is_some()));
}
