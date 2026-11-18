use super::*;
use crate::release::{repository_root, this_repository};
use crate::testutil::TempDir;

#[test]
fn license_files_are_found_by_name_and_read_once() {
    let temp = TempDir::new("notices-files");
    temp.write("LICENSE-MIT", "MIT text\r\nsecond line\r\n");
    temp.write("COPYING", "Copying text");
    temp.write("README.md", "not a license");
    temp.write("src/LICENSE", "inside a folder, not looked at");
    let texts = license_texts(temp.path());
    let names: Vec<&str> = texts.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["COPYING", "LICENSE-MIT"]);
    assert_eq!(texts[1].1, "MIT text\nsecond line");
}

#[test]
fn comp_02_the_notices_hold_every_license_text_the_parts_need() {
    let files = this_repository();
    let notices = &files.notices;
    let mut checked = 0;
    for part in files
        .list
        .parts
        .iter()
        .filter(|part| part.kind != Kind::File)
    {
        let texts = part
            .folder
            .as_deref()
            .map(license_texts)
            .unwrap_or_default();
        for (name, text) in &texts {
            checked += 1;
            assert!(
                notices.text.contains(text.as_str()),
                "the license text {name} of {} {} is not in the notices",
                part.name,
                part.version
            );
        }
        if texts.is_empty() {
            // A package that ships no license file is named in the notices, with a license
            // declared in its manifest; one that declares none is not allowed.
            assert!(
                part.license.is_some(),
                "{} {} has neither a license file nor a declared license",
                part.name,
                part.version
            );
            assert!(notices.without_text.contains(&label(part)));
        }
    }
    assert!(checked > 300, "only {checked} license texts were found");
    // The notices of code and data inside crates, and the data files that ship.
    for path in crate::release::parts_list::notice_files(&repository_root()).unwrap() {
        let text = fs::read_to_string(repository_root().join(&path)).unwrap();
        assert!(
            notices.text.contains(text.replace("\r\n", "\n").trim_end()),
            "{path} is not in the notices"
        );
    }
    assert!(notices.text.contains("breached-passwords.txt"));
}

#[test]
fn bad_example_a_package_without_a_license_file_is_named() {
    let mut list = this_repository().list.clone();
    list.parts.retain(|part| part.name == "alloc-stdlib");
    let register = crate::checks::dep_register::read_register(&repository_root()).unwrap();
    let notices = notices(&repository_root(), &list, &register).unwrap();
    assert!(
        notices
            .without_text
            .iter()
            .any(|name| name.starts_with("alloc-stdlib"))
    );
    assert!(notices.text.contains("ship no license file"));
}
