use std::collections::BTreeSet;

use serde_json::json;

use super::*;
use crate::release::parts_list::Program;
use crate::release::{compose, repository_root, this_repository, to_text};

fn small_list() -> PartsList {
    let part = |name: &str, version: &str, license: Option<&str>| Part {
        kind: Kind::Crate,
        name: name.to_owned(),
        version: version.to_owned(),
        license: license.map(str::to_owned),
        checksum: Some(Checksum {
            algorithm: Algorithm::Sha256,
            hex: "2866f0d976e76497480b853d5e258dab6e4b89ecfe991124e08f15dfa8fd03ad".to_owned(),
        }),
        purpose: None,
        depends_on: BTreeSet::new(),
        folder: None,
    };
    let mut serde = part("serde", "1.0.2", Some("MIT OR Apache-2.0"));
    serde
        .depends_on
        .insert("pkg:cargo/serde_derive@1.0.2".to_owned());
    PartsList {
        program: Program {
            name: "bugaboo".to_owned(),
            version: "0.5.1".to_owned(),
            license: "AGPL-3.0-or-later".to_owned(),
        },
        created: "2026-11-18T10:00:00Z".to_owned(),
        parts: vec![serde, part("serde_derive", "1.0.2", Some("MIT"))],
        direct: ["pkg:cargo/serde@1.0.2".to_owned()].into(),
    }
}

#[test]
fn a_small_list_is_valid_in_both_formats() {
    let list = small_list();
    assert_eq!(
        cyclonedx_errors(&cyclonedx(&list)).unwrap(),
        Vec::<String>::new()
    );
    assert_eq!(spdx_errors(&spdx(&list)).unwrap(), Vec::<String>::new());
}

#[test]
fn the_program_is_the_top_component_with_its_version_and_license() {
    let document = cyclonedx(&small_list());
    let top = &document["metadata"]["component"];
    assert_eq!(top["name"], "bugaboo");
    assert_eq!(top["version"], "0.5.1");
    assert_eq!(top["licenses"][0]["license"]["id"], "AGPL-3.0-or-later");
    let other = spdx(&small_list());
    assert_eq!(other["packages"][0]["licenseDeclared"], "AGPL-3.0-or-later");
    assert_eq!(other["packages"][0]["versionInfo"], "0.5.1");
}

#[test]
fn bad_example_a_component_without_a_name_fails_its_schema() {
    let mut document = cyclonedx(&small_list());
    document["components"][0]
        .as_object_mut()
        .unwrap()
        .remove("name");
    assert!(!cyclonedx_errors(&document).unwrap().is_empty());
    let mut document = spdx(&small_list());
    document["packages"][1]
        .as_object_mut()
        .unwrap()
        .remove("downloadLocation");
    assert!(!spdx_errors(&document).unwrap().is_empty());
    assert!(
        !cyclonedx_errors(&json!({"bomFormat": "other"}))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn comp_01_both_files_are_valid_against_their_schemas() {
    let files = this_repository();
    assert!(files.list.parts.len() > 100);
    assert_eq!(
        cyclonedx_errors(&files.cyclonedx).unwrap(),
        Vec::<String>::new()
    );
    assert_eq!(spdx_errors(&files.spdx).unwrap(), Vec::<String>::new());
}

#[test]
fn comp_01_two_runs_of_one_commit_write_the_same_bytes() {
    let first = this_repository();
    let second = compose(&repository_root()).unwrap();
    assert_eq!(to_text(&first.cyclonedx), to_text(&second.cyclonedx));
    assert_eq!(to_text(&first.spdx), to_text(&second.spdx));
    assert_eq!(first.notices.text, second.notices.text);
}

#[test]
fn the_files_are_named_for_the_version() {
    assert_eq!(cyclonedx_name("0.6.0"), "bugaboo-0.6.0.cdx.json");
    assert_eq!(spdx_name("0.6.0"), "bugaboo-0.6.0.spdx.json");
    assert_eq!(notices_name("0.6.0"), "bugaboo-0.6.0-notices.txt");
}
