//! The names the code declares (TXT-TERMS, glossary rule 4): the types, functions, modules,
//! constants and enum variants of Rust, the functions, classes, types, interfaces, enums and
//! constants of TypeScript, and the names of the folders and files that hold them. A name is cut
//! into words by the check itself (`ManyChangeRun` reads "many change run"), so this module only
//! says which names the code declares.
//!
//! Names that other people's code declares (the standard library, crates, npm packages) are never
//! read: only a declaration is a name, a call or a path to it is not. Generated files are not
//! read either, because what they declare comes from a source that is read.

use std::collections::BTreeSet;

use crate::web_code;
use anyhow::Context as _;

/// The folders whose code is read.
const CODE_FOLDERS: [&str; 3] = ["crates/", "xtask/", "web/"];

/// Folders in the code folders that are not read: other people's code, and what a program writes.
const NOT_READ_FOLDERS: [&str; 2] = ["generated", "node_modules"];

/// Files that are not read, whatever folder they are in: they are written by a program from a
/// source that is read.
const NOT_READ_FILES: [&str; 3] = ["generated.rs", "openapi.json", "schema.ts"];

/// One name the code declares, and what kind of thing it names.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Declared {
    /// `fn`, `struct`, `enum`, `variant`, `trait`, `type`, `mod`, `const`, `static`, `macro`,
    /// `class`, `interface`, `function`, `file` or `folder`.
    pub(super) what: &'static str,
    pub(super) name: String,
}

/// Whether `path` is a file of the code folders that this module reads.
pub(super) fn is_read(path: &str) -> bool {
    let file = path.rsplit('/').next().unwrap_or(path);
    CODE_FOLDERS.iter().any(|folder| path.starts_with(folder))
        && !NOT_READ_FILES.contains(&file)
        && !path.split('/').any(|part| NOT_READ_FOLDERS.contains(&part))
}

/// The names of the folders and files in `paths` (each path lists the folders it is in once, and
/// the file without its ending), as `(path of the folder or file, name)`.
pub(super) fn path_names(paths: &[String]) -> Vec<(String, Declared)> {
    let mut seen = BTreeSet::new();
    let mut found = Vec::new();
    for path in paths.iter().filter(|path| is_read(path)) {
        let parts: Vec<&str> = path.split('/').collect();
        for end in 1..parts.len() {
            let folder = parts[..end].join("/");
            if seen.insert(folder.clone()) {
                found.push((
                    folder,
                    Declared {
                        what: "folder",
                        name: parts[end - 1].to_owned(),
                    },
                ));
            }
        }
        if let Some(file) = parts.last() {
            let stem = file.split('.').next().unwrap_or(file);
            found.push((
                path.clone(),
                Declared {
                    what: "file",
                    name: stem.to_owned(),
                },
            ));
        }
    }
    found
}

/// The names a Rust file declares. A file that is not Rust is an error: it is named in the
/// finding, so the check never passes over a file it could not read.
pub(super) fn rust_names(text: &str) -> anyhow::Result<Vec<Declared>> {
    let file = syn::parse_file(text).context("is not Rust that this check can read")?;
    let mut found = Vec::new();
    items(&file.items, &mut found);
    Ok(found)
}

fn declare(found: &mut Vec<Declared>, what: &'static str, name: &syn::Ident) {
    let name = name.to_string();
    found.push(Declared {
        what,
        name: name.strip_prefix("r#").unwrap_or(&name).to_owned(),
    });
}

fn items(list: &[syn::Item], found: &mut Vec<Declared>) {
    for item in list {
        match item {
            syn::Item::Fn(item) => declare(found, "fn", &item.sig.ident),
            syn::Item::Struct(item) => declare(found, "struct", &item.ident),
            syn::Item::Union(item) => declare(found, "union", &item.ident),
            syn::Item::Enum(item) => {
                declare(found, "enum", &item.ident);
                for variant in &item.variants {
                    declare(found, "variant", &variant.ident);
                }
            }
            syn::Item::Trait(item) => {
                declare(found, "trait", &item.ident);
                for member in &item.items {
                    trait_member(member, found);
                }
            }
            syn::Item::Type(item) => declare(found, "type", &item.ident),
            syn::Item::Const(item) => declare(found, "const", &item.ident),
            syn::Item::Static(item) => declare(found, "static", &item.ident),
            syn::Item::Mod(item) => {
                declare(found, "mod", &item.ident);
                if let Some((_, inside)) = &item.content {
                    items(inside, found);
                }
            }
            // The members of `impl Trait for Type` are named by the trait: by this code, where the
            // trait is read, or by other people's code, which is not.
            syn::Item::Impl(item) if item.trait_.is_none() => {
                for member in &item.items {
                    impl_member(member, found);
                }
            }
            syn::Item::Macro(item) => {
                if let Some(name) = &item.ident {
                    declare(found, "macro", name);
                }
                in_macro(&item.mac.tokens.to_string(), found);
            }
            _ => {}
        }
    }
}

fn trait_member(member: &syn::TraitItem, found: &mut Vec<Declared>) {
    match member {
        syn::TraitItem::Fn(item) => declare(found, "fn", &item.sig.ident),
        syn::TraitItem::Const(item) => declare(found, "const", &item.ident),
        syn::TraitItem::Type(item) => declare(found, "type", &item.ident),
        syn::TraitItem::Macro(item) => in_macro(&item.mac.tokens.to_string(), found),
        _ => {}
    }
}

fn impl_member(member: &syn::ImplItem, found: &mut Vec<Declared>) {
    match member {
        syn::ImplItem::Fn(item) => declare(found, "fn", &item.sig.ident),
        syn::ImplItem::Const(item) => declare(found, "const", &item.ident),
        syn::ImplItem::Type(item) => declare(found, "type", &item.ident),
        syn::ImplItem::Macro(item) => in_macro(&item.mac.tokens.to_string(), found),
        _ => {}
    }
}

/// The words before which a macro call may declare a name.
const DECLARING: [(&str, &str); 8] = [
    ("fn", "fn"),
    ("struct", "struct"),
    ("enum", "enum"),
    ("trait", "trait"),
    ("type", "type"),
    ("mod", "mod"),
    ("const", "const"),
    ("static", "static"),
];

/// The names that a macro call declares, as far as its tokens show: the word after `fn`,
/// `struct`, `enum`, `trait`, `type`, `mod`, `const` or `static`, and a word before `=>` (the
/// variants of the macros that list words).
fn in_macro(tokens: &str, found: &mut Vec<Declared>) {
    let words = super::words(tokens, true);
    for pair in words.windows(2) {
        let [(keyword, _), (name, at)] = pair else {
            continue;
        };
        if let Some((_, what)) = DECLARING.iter().find(|(text, _)| text == keyword) {
            found.push(Declared {
                what,
                name: (*name).to_owned(),
            });
        }
        if tokens[at + name.len()..].trim_start().starts_with("=>") {
            found.push(Declared {
                what: "variant",
                name: (*name).to_owned(),
            });
        }
    }
    // The first word of the call can stand before `=>` without a word before it.
    if let Some((name, at)) = words.first()
        && tokens[at + name.len()..].trim_start().starts_with("=>")
    {
        found.push(Declared {
            what: "variant",
            name: (*name).to_owned(),
        });
    }
}

/// The names a TypeScript file declares: after `function`, `class`, `interface`, `enum`,
/// `namespace`, `const`, `let` and `var`, and a `type` alias. A destructuring pattern declares the
/// names of other code, so it is not read.
pub(super) fn web_names(text: &str) -> Vec<Declared> {
    const KEYWORDS: [(&str, &str); 9] = [
        ("function", "function"),
        ("class", "class"),
        ("interface", "interface"),
        ("enum", "enum"),
        ("namespace", "namespace"),
        ("type", "type"),
        ("const", "const"),
        ("let", "let"),
        ("var", "var"),
    ];
    let code = web_code::strip(text);
    let words = super::words(&code, true);
    let mut found = Vec::new();
    for pair in words.windows(2) {
        let [(keyword, from), (name, at)] = pair else {
            continue;
        };
        let Some((_, what)) = KEYWORDS.iter().find(|(text, _)| text == keyword) else {
            continue;
        };
        let between = &code[from + keyword.len()..*at];
        // `function* name`, `class name`: only blanks and a star may stand between.
        if between.chars().all(|c| c.is_whitespace() || c == '*')
            && !between.is_empty()
            && !matches!(*name, "extends" | "implements" | "of" | "in")
        {
            found.push(Declared {
                what,
                name: (*name).to_owned(),
            });
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(found: &[Declared]) -> Vec<(&'static str, &str)> {
        found.iter().map(|d| (d.what, d.name.as_str())).collect()
    }

    #[test]
    fn rust_declarations_are_names_and_calls_are_not() {
        let found = rust_names(
            "pub struct IssueAssignee;
             pub enum Mode { Quick, SlowOne }
             pub trait Maker { fn build(&self); const WIDTH: u8; type Out; }
             mod inner { pub fn make_project() {} }
             impl IssueAssignee { pub fn new() -> Self { let x = 1; \"A\".make_ascii_lowercase(); Self } }
             const LIMIT: u8 = 1;
             static GLOBAL: u8 = 2;
             type Alias = u8;
             macro_rules! shout { () => {}; }
             impl MakeWriter for IssueAssignee { fn make_writer(&self) {} }",
        )
        .unwrap();
        let names = names(&found);
        for wanted in [
            ("struct", "IssueAssignee"),
            ("enum", "Mode"),
            ("variant", "SlowOne"),
            ("trait", "Maker"),
            ("fn", "build"),
            ("const", "WIDTH"),
            ("type", "Out"),
            ("mod", "inner"),
            ("fn", "make_project"),
            ("fn", "new"),
            ("const", "LIMIT"),
            ("static", "GLOBAL"),
            ("type", "Alias"),
            ("macro", "shout"),
        ] {
            assert!(names.contains(&wanted), "{wanted:?} in {names:?}");
        }
        // A call of a name from the standard library declares nothing, and neither does a local.
        assert!(
            !names
                .iter()
                .any(|(_, name)| *name == "make_ascii_lowercase")
        );
        assert!(!names.iter().any(|(_, name)| *name == "x"));
        // The methods of a trait impl are named by the trait.
        assert!(!names.iter().any(|(_, name)| *name == "make_writer"));
    }

    #[test]
    fn what_a_macro_declares_is_read_from_its_tokens() {
        let found = rust_names(
            "words! {
                pub enum Status { Queued => \"queued\", ManyChanges => \"x\" }
             }
             permissions! {
                MakeIssues => (\"make-issues\", Issues),
             }",
        )
        .unwrap();
        let names = names(&found);
        for wanted in [
            ("enum", "Status"),
            ("variant", "Queued"),
            ("variant", "ManyChanges"),
            ("variant", "MakeIssues"),
        ] {
            assert!(names.contains(&wanted), "{wanted:?} in {names:?}");
        }
    }

    #[test]
    fn rust_that_cannot_be_read_is_an_error_and_not_silence() {
        assert!(rust_names("fn {").is_err());
    }

    #[test]
    fn typescript_declarations_are_names_and_imports_are_not() {
        let found = web_names(
            "import { makeProject as other } from \"./x.ts\";
             // function commented() {}
             export function IssueList() {}
             export async function* streamed() {}
             export const useThing = () => 1;
             let counter = 0;
             class Shape extends Base {}
             export interface Props { a: string }
             export type Kind = \"a\";
             enum Direction { Up }
             const { fromElsewhere } = something();
             const text = \"function inString() {}\";",
        );
        let names = names(&found);
        for wanted in [
            ("function", "IssueList"),
            ("function", "streamed"),
            ("const", "useThing"),
            ("let", "counter"),
            ("class", "Shape"),
            ("interface", "Props"),
            ("type", "Kind"),
            ("enum", "Direction"),
        ] {
            assert!(names.contains(&wanted), "{wanted:?} in {names:?}");
        }
        for not_wanted in ["makeProject", "other", "commented", "fromElsewhere", "Base"] {
            assert!(
                !names.iter().any(|(_, name)| *name == not_wanted),
                "{not_wanted} in {names:?}"
            );
        }
    }

    #[test]
    fn folders_and_files_are_names_once_and_other_code_is_left_out() {
        let paths = [
            "crates/bb-app/src/many_changes/run.rs".to_owned(),
            "crates/bb-app/src/many_changes/apply.rs".to_owned(),
            "web/app/src/features/many-changes/index.ts".to_owned(),
            "web/packages/messages/src/generated/labels.ts".to_owned(),
            "docs/page.md".to_owned(),
        ];
        let found = path_names(&paths);
        let shown: Vec<(&str, &str)> = found
            .iter()
            .map(|(_, d)| (d.what, d.name.as_str()))
            .collect();
        assert_eq!(
            shown
                .iter()
                .filter(|name| **name == ("folder", "many_changes"))
                .count(),
            1
        );
        assert!(shown.contains(&("file", "run")));
        assert!(shown.contains(&("file", "index")));
        assert!(shown.contains(&("folder", "many-changes")));
        // Generated code and what is outside the code folders are not read.
        assert!(
            !shown
                .iter()
                .any(|(_, name)| *name == "labels" || *name == "page")
        );
    }
}
