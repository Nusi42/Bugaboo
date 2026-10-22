//! The tests of the one required-tests implementation.

use super::*;

/// A crate with a root that declares `mod tests;`, and the text of that module.
fn crate_with(tests: &str) -> Vec<Source> {
    vec![
        ("crates/x/src/lib.rs".to_owned(), "mod tests;\n".to_owned()),
        ("crates/x/src/tests.rs".to_owned(), tests.to_owned()),
    ]
}

const NAME: &str = "x_01_a_thing_works";

fn found(sources: &[Source]) -> usize {
    evaluate(sources, &[Required::anywhere(NAME)], "").len()
}

#[test]
fn good_example_a_live_test_in_a_declared_module() {
    let text = format!("#[tokio::test]\nasync fn {NAME}() {{}}\n");
    assert_eq!(found(&crate_with(&text)), 0);
    let plain = format!("#[test]\nfn {NAME}() {{}}\n");
    assert_eq!(found(&crate_with(&plain)), 0);
}

#[test]
fn bad_example_a_test_in_a_module_nobody_declares() {
    let text = format!("#[test]\nfn {NAME}() {{}}\n");
    let sources = vec![
        ("crates/x/src/lib.rs".to_owned(), "mod other;\n".to_owned()),
        ("crates/x/src/tests.rs".to_owned(), text),
    ];
    assert_eq!(found(&sources), 1);
}

#[test]
fn bad_example_a_module_declared_only_behind_a_cfg_or_only_by_a_dead_file() {
    let text = format!("#[test]\nfn {NAME}() {{}}\n");
    for root in [
        "#[cfg(feature = \"later\")]\nmod tests;\n",
        "#[cfg(any())]\nmod tests;\n",
    ] {
        let sources = vec![
            ("crates/x/src/lib.rs".to_owned(), root.to_owned()),
            ("crates/x/src/tests.rs".to_owned(), text.clone()),
        ];
        assert_eq!(found(&sources), 1, "{root}");
    }
    // A file that only a file nobody declares declares.
    let sources = vec![
        ("crates/x/src/lib.rs".to_owned(), String::new()),
        ("crates/x/src/dead.rs".to_owned(), "mod tests;\n".to_owned()),
        ("crates/x/src/dead/tests.rs".to_owned(), text),
    ];
    assert_eq!(found(&sources), 1);
}

#[test]
fn good_example_modules_reached_by_a_path_attribute_a_mod_rs_or_a_tests_folder() {
    let text = format!("#[test]\nfn {NAME}() {{}}\n");
    let by_path = vec![
        (
            "crates/x/src/lib.rs".to_owned(),
            "#[cfg(test)]\n#[path = \"deep/tests.rs\"]\nmod tests;\n".to_owned(),
        ),
        ("crates/x/src/deep/tests.rs".to_owned(), text.clone()),
    ];
    assert_eq!(found(&by_path), 0);
    let nested = vec![
        ("crates/x/src/lib.rs".to_owned(), "mod a;\n".to_owned()),
        ("crates/x/src/a/mod.rs".to_owned(), "mod b;\n".to_owned()),
        ("crates/x/src/a/b.rs".to_owned(), "mod c;\n".to_owned()),
        ("crates/x/src/a/b/c.rs".to_owned(), text.clone()),
    ];
    assert_eq!(found(&nested), 0);
    let integration = vec![
        (
            "crates/x/tests/all.rs".to_owned(),
            "mod cases;\n".to_owned(),
        ),
        ("crates/x/tests/cases.rs".to_owned(), text),
    ];
    assert_eq!(found(&integration), 0);
}

#[test]
fn bad_example_a_test_that_is_no_test_any_more_or_hides_in_a_comment() {
    for text in [
        format!("fn {NAME}() {{}}\n"),
        format!("// #[test]\n// fn {NAME}() {{}}\n"),
        String::new(),
    ] {
        assert_eq!(found(&crate_with(&text)), 1, "{text}");
    }
}

#[test]
fn bad_example_a_test_that_is_switched_off() {
    for attributes in [
        "#[test]\n#[ignore]",
        "#[ignore]\n#[test]",
        "#[test]\n#[ignore = \"later\"]",
        "#[test]\n#[ignore =\n    \"later\"]",
        "#[test]\n#[cfg_attr(windows, ignore)]",
        "#[test]\n/// Later.\n#[ignore]",
        "#[test]\n#[cfg_attr(\n    windows,\n    ignore\n)]",
        "#[cfg(any())]\n#[test]",
        "#[test]\n#[cfg(not(test))]",
    ] {
        let text = format!("{attributes}\nfn {NAME}() {{}}\n");
        assert_eq!(found(&crate_with(&text)), 1, "{attributes}");
    }
    for block in [
        "#[cfg(any())]\nmod later {\n#[test]\nfn NAME() {}\n}",
        "#[cfg(any())]\nmod later {\nmod inner {\n#[test]\nfn NAME() {}\n}\n}",
    ] {
        let text = block.replace("NAME", NAME);
        assert_eq!(found(&crate_with(&text)), 1, "{block}");
    }
}

#[test]
fn good_example_doc_comments_cfg_test_and_other_attributes_are_fine() {
    for text in [
        format!("/// What it proves.\n#[tokio::test]\n#[doc(hidden)]\nasync fn {NAME}() {{}}\n"),
        format!("#[cfg(test)]\nmod inner {{\n#[test]\nfn {NAME}() {{ if true {{}} }}\n}}\n"),
        format!("mod ordinary {{\n#[test]\nfn {NAME}() {{ if true {{}} }}\n}}\n"),
    ] {
        assert_eq!(found(&crate_with(&text)), 0, "{text}");
    }
}

#[test]
fn good_example_a_test_attribute_with_arguments_is_still_a_test() {
    for attribute in [
        "#[tokio::test(flavor = \"multi_thread\", worker_threads = 4)]",
        "#[tokio::test(flavor = \"multi_thread\")]",
        "#[sqlx::test]",
    ] {
        let text = format!(
            "{attribute}
async fn {NAME}() {{}}
"
        );
        assert_eq!(found(&crate_with(&text)), 0, "{attribute}");
    }
    // A condition on `test` is no test attribute.
    let only_cfg = format!(
        "#[cfg(test)]
fn {NAME}() {{}}
"
    );
    assert_eq!(found(&crate_with(&only_cfg)), 1);
}

#[test]
fn bad_example_a_module_declared_in_a_string_or_a_comment_is_no_module() {
    let text = format!(
        "#[test]
fn {NAME}() {{}}
"
    );
    for root in [
        "const TEXT: &str = \"mod tests;\";
",
        "// mod tests;
const TEXT: &str = r#\"
mod tests;
\"#;
",
    ] {
        let sources = vec![
            ("crates/x/src/lib.rs".to_owned(), root.to_owned()),
            ("crates/x/src/tests.rs".to_owned(), text.clone()),
        ];
        assert_eq!(found(&sources), 1, "{root}");
    }
}

#[test]
fn good_and_bad_example_a_module_inside_an_inline_module_is_in_a_folder_of_its_name() {
    let text = format!(
        "#[test]
fn {NAME}() {{}}
"
    );
    let sources = |at: &str| {
        vec![
            (
                "crates/x/src/lib.rs".to_owned(),
                "mod outer {
mod inner {
    mod tests;
}
}
"
                .to_owned(),
            ),
            (at.to_owned(), text.clone()),
        ]
    };
    assert_eq!(found(&sources("crates/x/src/outer/inner/tests.rs")), 0);
    assert_eq!(found(&sources("crates/x/src/tests.rs")), 1);
    // A path attribute inside an inline module is relative to the folders of the modules.
    let with_path = |at: &str| {
        vec![
            (
                "crates/x/src/lib.rs".to_owned(),
                "mod outer {
#[path = \"deep/tests.rs\"]
mod tests;
}
"
                .to_owned(),
            ),
            (at.to_owned(), text.clone()),
        ]
    };
    assert_eq!(found(&with_path("crates/x/src/outer/deep/tests.rs")), 0);
    assert_eq!(found(&with_path("crates/x/src/deep/tests.rs")), 1);
}

#[test]
fn a_test_has_to_be_where_the_list_says() {
    let text = format!("#[test]\nfn {NAME}() {{}}\n");
    let sources = crate_with(&text);
    let there = [Required::anywhere(NAME).within("crates/x/src/tests.rs")];
    assert!(evaluate(&sources, &there, "").is_empty());
    let elsewhere = [Required::anywhere(NAME).within("crates/y/")];
    assert_eq!(evaluate(&sources, &elsewhere, "").len(), 1);
}

#[test]
fn a_test_below_a_folder_is_in_that_folder_and_not_in_a_folder_named_like_it() {
    let text = format!("#[test]\nfn {NAME}() {{}}\n");
    let sources = |at: &str| {
        vec![
            (
                "crates/x/src/lib.rs".to_owned(),
                "mod editing_old;\nmod editing;\n".to_owned(),
            ),
            (at.to_owned(), text.clone()),
        ]
    };
    let in_folder = [Required::anywhere(NAME).within("crates/x/src/editing")];
    assert!(evaluate(&sources("crates/x/src/editing/mod.rs"), &in_folder, "").is_empty());
    assert_eq!(
        evaluate(&sources("crates/x/src/editing_old.rs"), &in_folder, "").len(),
        1
    );
}

#[test]
fn bad_example_a_test_file_in_a_listed_folder_that_no_mod_reaches() {
    let text = format!("#[test]\nfn {NAME}() {{}}\n");
    let folder = [Required::anywhere(NAME).within("crates/x/src/tests")];
    let mut sources = vec![
        ("crates/x/src/lib.rs".to_owned(), "mod tests;\n".to_owned()),
        (
            "crates/x/src/tests/mod.rs".to_owned(),
            "mod kept;\n".to_owned(),
        ),
        ("crates/x/src/tests/kept.rs".to_owned(), text),
    ];
    assert!(evaluate(&sources, &folder, "").is_empty());
    // A file nobody declares, with a test of another name.
    sources.push((
        "crates/x/src/tests/stale_views.rs".to_owned(),
        "#[tokio::test]\nasync fn other() {}\n".to_owned(),
    ));
    let found = evaluate(&sources, &folder, "");
    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains("no `mod` leads"));
    // One declared only behind a cfg is no better.
    sources[1].1 = "mod kept;\n#[cfg(any())]\nmod stale_views;\n".to_owned();
    assert_eq!(evaluate(&sources, &folder, "").len(), 1);
    // Declared: fine.
    sources[1].1 = "mod kept;\nmod stale_views;\n".to_owned();
    assert!(evaluate(&sources, &folder, "").is_empty());
}

#[test]
fn bad_and_good_example_an_inner_cfg_attribute_switches_off_the_tests_of_its_scope() {
    for text in [
        format!("#![cfg(any())]\n#[test]\nfn {NAME}() {{}}\n"),
        format!("#![warn(dead_code)]\n#![cfg(feature = \"later\")]\n#[test]\nfn {NAME}() {{}}\n"),
        format!("mod m {{\n    #![cfg(any())]\n    #[test]\n    fn {NAME}() {{}}\n}}\n"),
        format!(
            "mod m {{\n    fn before() {{}}\n    mod n {{\n        #![cfg(any())]\n        #[test]\n        fn {NAME}() {{}}\n    }}\n}}\n"
        ),
    ] {
        assert_eq!(found(&crate_with(&text)), 1, "{text}");
    }
    for text in [
        format!("#![cfg(test)]\n#[test]\nfn {NAME}() {{}}\n"),
        format!("#![warn(dead_code)]\n#[test]\nfn {NAME}() {{}}\n"),
        format!("mod m {{\n    #![warn(dead_code)]\n    #[test]\n    fn {NAME}() {{}}\n}}\n"),
    ] {
        assert_eq!(found(&crate_with(&text)), 0, "{text}");
    }
}

#[test]
fn both_databases_look_at_the_test_and_not_at_the_one_after_it() {
    let text = format!(
        "mod tests {{\n    #[test]\n    fn {NAME}() {{\n        body();\n    }}\n\n    #[test]\n    fn other() {{\n        each_database(\"x\");\n    }}\n}}\n"
    );
    assert_eq!(
        evaluate(&crate_with(&text), &[Required::both(NAME)], "").len(),
        1
    );
    let both = format!(
        "mod tests {{\n    #[test]\n    fn {NAME}() {{\n        each_database(\"x\");\n    }}\n}}\n"
    );
    assert!(evaluate(&crate_with(&both), &[Required::both(NAME)], "").is_empty());
}
