use std::path::PathBuf;

use super::*;
use crate::checks::dep_register::parse_register;
use crate::release::parts_list::{Algorithm, Checksum, Part, Program, lock_checksums};
use crate::release::{repository_root, this_repository};

fn fixture(kind: &str) -> (Names, Names) {
    let folder: PathBuf = repository_root()
        .join("tools/checks/fixtures/DEP-SBOM")
        .join(kind);
    let lock = fs::read_to_string(folder.join("lockfile.toml")).unwrap();
    let expected: Names = lock_checksums(&lock).unwrap().into_keys().collect();
    let document: Value =
        serde_json::from_str(&fs::read_to_string(folder.join("parts-list.cdx.json")).unwrap())
            .unwrap();
    (expected, cyclonedx_names(&document))
}

#[test]
fn bad_example_a_lockfile_with_a_crate_the_list_leaves_out() {
    let (expected, actual) = fixture("bad");
    let findings = findings_of("CycloneDX file", &expected, &actual);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("itoa 1.0.0"));
    assert!(findings[0].message.contains("missing"));
}

#[test]
fn good_example_a_list_that_matches_the_lockfile() {
    let (expected, actual) = fixture("good");
    assert!(findings_of("CycloneDX file", &expected, &actual).is_empty());
}

#[test]
fn bad_example_an_entry_without_a_package_is_found() {
    let (expected, mut actual) = fixture("good");
    actual.insert(("ghost".to_owned(), "9.9.9".to_owned()));
    let findings = findings_of("SPDX file", &expected, &actual);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("names no package"));
}

fn part(license: Option<&str>, checksum: bool) -> Part {
    Part {
        kind: Kind::Crate,
        name: "x".to_owned(),
        version: "1".to_owned(),
        license: license.map(str::to_owned),
        checksum: checksum.then(|| Checksum {
            algorithm: Algorithm::Sha256,
            hex: "ab".to_owned(),
        }),
        purpose: None,
        depends_on: BTreeSet::new(),
        folder: None,
    }
}

fn list_of(parts: Vec<Part>) -> PartsList {
    PartsList {
        program: Program {
            name: "bugaboo".to_owned(),
            version: "1".to_owned(),
            license: "AGPL-3.0-or-later".to_owned(),
        },
        created: String::new(),
        parts,
        direct: BTreeSet::new(),
    }
}

#[test]
fn bad_example_a_part_without_license_or_checksum() {
    assert_eq!(unfinished_parts(&list_of(vec![part(None, true)])).len(), 1);
    assert_eq!(
        unfinished_parts(&list_of(vec![part(Some("MIT"), false)])).len(),
        1
    );
    assert_eq!(
        unfinished_parts(&list_of(vec![part(Some("SEE LICENSE IN x"), true)])).len(),
        1
    );
    assert!(unfinished_parts(&list_of(vec![part(Some("MIT OR Apache-2.0"), true)])).is_empty());
}

#[test]
fn bad_example_a_font_file_without_an_entry_is_found() {
    let register = parse_register(
        "| Data | Source | License | Ships? | Purpose |\n|---|---|---|---|---|\n| `web/app/public/ok.woff2` | x | OFL-1.1 | yes | y |\n",
    );
    let files = [
        "web/app/public/ok.woff2",
        "web/app/dist/assets/inter.woff2",
        "web/app/src/a.ts",
    ]
    .map(str::to_owned);
    assert_eq!(
        unlisted_fonts(&files, &register),
        ["web/app/dist/assets/inter.woff2"]
    );
}

#[test]
fn bad_example_an_icon_set_the_list_leaves_out_is_found() {
    let mut lucide = part(Some("ISC"), true);
    lucide.kind = Kind::Npm;
    lucide.name = "lucide-react".to_owned();
    let list = list_of(vec![lucide]);
    let wanted: BTreeSet<String> = ["lucide-react", "@fortawesome/fontawesome-free"]
        .map(str::to_owned)
        .into();
    assert_eq!(
        unlisted_icon_sets(&wanted, &list),
        ["@fortawesome/fontawesome-free"]
    );
}

#[test]
fn comp_01_no_font_or_icon_set_ships_without_an_entry() {
    let root = repository_root();
    let register = read_register(&root).unwrap();
    let files = crate::repo::Repo::at(root.clone()).all_files().unwrap();
    let list = &this_repository().list;
    let found = shipped_without_entry(&root, &files, &register, list).unwrap();
    assert!(
        found.is_empty(),
        "{:?}",
        found.iter().map(|f| &f.message).collect::<Vec<_>>()
    );
    let lucide = list
        .parts
        .iter()
        .find(|part| part.name == "lucide-react")
        .unwrap();
    assert_eq!(lucide.license.as_deref(), Some("ISC"));
}

#[test]
fn the_repository_passes() {
    let files = this_repository();
    let formats_ok = release::schema_errors(files).unwrap();
    assert!(formats_ok.is_empty(), "{formats_ok:?}");
    assert!(unfinished_parts(&files.list).is_empty());
    assert!(compare(&files.list, &files.cyclonedx, &files.spdx).is_empty());
}
