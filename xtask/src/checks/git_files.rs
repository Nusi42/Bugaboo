//! GIT-FILES: no large binaries and no CRLF line endings committed (coding-standards §7).
//!
//! Line endings are judged as Git stores them (the index), so `.gitattributes` decides: a file
//! that is stored with LF and checked out with CRLF (for example `*.ps1`) is fine.

use globset::{Glob, GlobSet, GlobSetBuilder};
use std::fs;

use super::{Check, Context, Finding};
use crate::repo::Repo;

/// The largest file that may be committed, in bytes.
const MAX_BYTES: u64 = 1024 * 1024;

/// Files that may be larger than [`MAX_BYTES`]. Add a glob with a reason in the comment, and
/// only for files that cannot be generated or split.
const LARGE_ALLOWED: &[&str] = &[];

pub(crate) struct GitFiles;

impl Check for GitFiles {
    fn id(&self) -> &'static str {
        "GIT-FILES"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let only = context.files_for(self.id())?;
        check_root(context.repo, only.as_deref())
    }
}

fn check_root(repo: &Repo, only: Option<&[String]>) -> anyhow::Result<Vec<Finding>> {
    let allowed = allow_list()?;
    let mut findings = Vec::new();
    let root = repo.real_root();
    for entry in index_entries(repo)? {
        if only.is_some_and(|files| !files.contains(&entry.path)) {
            continue;
        }
        let Ok(metadata) = fs::metadata(root.join(&entry.path)) else {
            continue;
        };
        if metadata.len() > MAX_BYTES && !allowed.is_match(&entry.path) {
            findings.push(Finding::at(
                &entry.path,
                format!(
                    "is {} KB; files over {} KB are not allowed. Keep big files out of Git (generate or download them).",
                    metadata.len() / 1024,
                    MAX_BYTES / 1024
                ),
            ));
        }
        if entry.index_has_crlf {
            findings.push(Finding::at(
                &entry.path,
                "has CRLF line endings in the repository. Convert it to LF (for example `git add --renormalize <file>`), \
                 or mark the file type in .gitattributes if Windows needs CRLF there.",
            ));
        }
    }
    Ok(findings)
}

fn allow_list() -> anyhow::Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in LARGE_ALLOWED {
        builder.add(Glob::new(pattern)?);
    }
    Ok(builder.build()?)
}

struct IndexEntry {
    path: String,
    index_has_crlf: bool,
}

/// Every tracked file with what Git stored for its line endings (`git ls-files --eol`).
fn index_entries(repo: &Repo) -> anyhow::Result<Vec<IndexEntry>> {
    let out = repo.git_bytes(&["-c", "core.quotepath=false", "ls-files", "--eol", "-z"])?;
    Ok(String::from_utf8_lossy(&out)
        .split('\0')
        .filter_map(|record| record.split_once('\t'))
        .map(|(info, path)| IndexEntry {
            path: path.to_owned(),
            index_has_crlf: info
                .split_whitespace()
                .next()
                .is_some_and(|index| index == "i/crlf" || index == "i/mixed"),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn commit_all(temp: &TempDir) {
        assert!(temp.git(&["init", "--quiet"]));
        assert!(temp.git(&["add", "--force", "."]));
        assert!(temp.git(&["commit", "--quiet", "-m", "chore: test"]));
    }

    #[test]
    fn bad_example_crlf() {
        let temp = TempDir::new("files-crlf");
        temp.write("a.txt", "one\r\ntwo\r\n");
        commit_all(&temp);
        let findings = check_root(&Repo::at(temp.path().to_path_buf()), None).unwrap();
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("CRLF"));
    }

    #[test]
    fn bad_example_large_file() {
        let temp = TempDir::new("files-large");
        temp.write("big.bin", vec![b'x'; 1_100_000]);
        commit_all(&temp);
        let findings = check_root(&Repo::at(temp.path().to_path_buf()), None).unwrap();
        assert!(findings.iter().any(|f| f.message.contains("KB")));
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("files-good");
        temp.write("a.txt", "one\ntwo\n");
        temp.write("small.bin", vec![0_u8; 1000]);
        commit_all(&temp);
        assert!(
            check_root(&Repo::at(temp.path().to_path_buf()), None)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn good_example_attributes_decide_for_crlf_checkouts() {
        let temp = TempDir::new("files-attributes");
        temp.write(".gitattributes", "*.ps1 text eol=crlf\n");
        temp.write("run.ps1", "echo hi\r\n");
        assert!(temp.git(&["init", "--quiet"]));
        assert!(temp.git(&["add", "."]));
        assert!(temp.git(&["commit", "--quiet", "-m", "chore: test"]));
        assert!(
            check_root(&Repo::at(temp.path().to_path_buf()), None)
                .unwrap()
                .is_empty()
        );
    }
}
