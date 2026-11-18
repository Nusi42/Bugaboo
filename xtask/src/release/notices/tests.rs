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
            // No license file in the package: the standard text of the declared license stands in,
            // with the authors of the package as the copyright line.
            let standard = standard_licenses(part.license.as_deref().unwrap_or_default());
            assert!(
                !standard.is_empty(),
                "{} {} has no license text",
                part.name,
                part.version
            );
            let text = standard_text(standard[0], &part.authors, &part.name).unwrap();
            assert!(notices.text.contains(&text));
        }
    }
    assert_eq!(
        notices.without_text,
        Vec::<String>::new(),
        "every listed crate has license text"
    );
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
fn the_standard_text_names_the_authors_and_the_first_license_that_has_one() {
    let authors = [
        "Ada Lovelace <ada@example.test>".to_owned(),
        "Bob".to_owned(),
    ];
    let mit = standard_text("MIT", &authors, "x").unwrap();
    assert!(mit.contains("Copyright (c) Ada Lovelace, Bob"));
    assert!(
        !mit.contains('<'),
        "no placeholder or e-mail address is left: {mit}"
    );
    let bsd = standard_text("BSD-3-Clause", &[], "thing").unwrap();
    assert!(bsd.contains("Copyright (c) the authors of thing"));
    assert_eq!(standard_licenses("MIT OR Apache-2.0"), ["MIT"]);
    assert_eq!(standard_licenses("(Apache-2.0 OR MIT)"), ["Apache-2.0"]);
    assert_eq!(standard_licenses("MIT/Apache-2.0"), ["MIT"]);
    assert_eq!(
        standard_licenses("MIT AND BSD-3-Clause"),
        ["MIT", "BSD-3-Clause"]
    );
    assert_eq!(standard_licenses("Unicode-3.0"), Vec::<&str>::new());
    assert!(
        standard_text("MPL-2.0", &[], "x")
            .unwrap()
            .contains("Mozilla Public License")
    );
}

#[test]
fn bad_example_a_package_without_a_file_or_a_standard_text_is_named() {
    let mut list = this_repository().list.clone();
    list.parts.retain(|part| part.name == "alloc-stdlib");
    for part in &mut list.parts {
        part.license = Some("LicenseRef-unknown".to_owned());
    }
    let register = crate::checks::dep_register::read_register(&repository_root()).unwrap();
    let notices = notices(&repository_root(), &list, &register).unwrap();
    assert_eq!(notices.without_text.len(), list.parts.len());
    assert!(notices.text.contains("ship no license file"));
}
