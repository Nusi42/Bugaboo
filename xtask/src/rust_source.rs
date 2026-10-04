//! Reading the Rust source of the product crates, for the checks and generators that look at
//! code instead of running a tool on it.

use std::path::Path;

use anyhow::Context as _;

use crate::repo::Repo;

/// One Rust source file: its repository-relative path and its text.
pub(crate) type Source = (String, String);

/// Every Rust file of the product crates, except integration tests and their examples.
pub(crate) fn read_product_sources(repo: &Repo) -> anyhow::Result<Vec<Source>> {
    let mut sources = Vec::new();
    for file in repo.all_files()? {
        let is_product_code = file.starts_with("crates/")
            && Path::new(&file)
                .extension()
                .is_some_and(|extension| extension == "rs")
            && !file.split('/').any(|part| part == "tests");
        if is_product_code {
            let text = std::fs::read_to_string(repo.path(&file))
                .with_context(|| format!("could not read {file}"))?;
            sources.push((file, text.replace("\r\n", "\n")));
        }
    }
    Ok(sources)
}

/// The part of a file before its unit tests. By convention tests are the last module of a file,
/// so what comes before `#[cfg(test)]` is the code that ships.
pub(crate) fn before_tests(text: &str) -> &str {
    text.find("#[cfg(test)]").map_or(text, |at| &text[..at])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_ends_where_the_tests_begin() {
        assert_eq!(
            before_tests("fn a() {}\n#[cfg(test)]\nmod tests {}\n"),
            "fn a() {}\n"
        );
        assert_eq!(before_tests("fn a() {}\n"), "fn a() {}\n");
    }
}
