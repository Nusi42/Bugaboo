//! Reading what the history of Git holds: commit messages, tags and what unpushed commits add.

use super::*;

/// The records of a Git listing that ends each record with the separator `0x1e`.
pub(super) fn records(listing: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(listing)
        .split('\u{1e}')
        .map(|record| record.trim_start().to_owned())
        .filter(|record| !record.is_empty())
        .collect()
}

/// What a push can send: the messages of the commits on every branch, on every tag and on `HEAD`,
/// the messages of annotated tags, the names of branches and tags, and the names and contents of
/// the files that the commits not on a remote add or change.
pub(super) fn scan_history(
    repo: &Repo,
    words: &PrivateWords,
    exempt: &[Exempt],
    skipped: &str,
) -> anyhow::Result<Vec<Finding>> {
    if repo
        .git_bytes(&["rev-parse", "--verify", "--quiet", "HEAD"])
        .is_err()
    {
        return Ok(Vec::new());
    }
    let commits = repo.git_bytes(&[
        "log",
        "--format=%h%x1f%B%x1e",
        "--branches",
        "--tags",
        "HEAD",
    ])?;
    let tags = repo.git_bytes(&[
        "for-each-ref",
        "--format=%(objecttype)%1f%(refname:short)%1f%(contents)%1e",
        "refs/tags",
    ])?;
    let branches =
        repo.git_bytes(&["for-each-ref", "--format=%(refname:short)%1e", "refs/heads"])?;
    // Each tag as (kind of object, name, message).
    let tags: Vec<(String, String, String)> = records(&tags)
        .iter()
        .map(|record| {
            let mut parts = record.splitn(3, '\u{1f}').map(str::to_owned);
            (
                parts.next().unwrap_or_default(),
                parts.next().unwrap_or_default(),
                parts.next().unwrap_or_default(),
            )
        })
        .collect();
    let mut findings = Vec::new();
    for record in records(&commits) {
        if let Some((hash, message)) = record.split_once('\u{1f}') {
            findings.extend(scan_message(&format!("commit {hash}"), message, words));
        }
    }
    for (kind, name, message) in &tags {
        if kind == "tag" {
            let place = format!("tag {}", masked(name, words));
            findings.extend(scan_message(&place, message, words));
        }
    }
    for (kind, name) in records(&branches)
        .iter()
        .map(|name| ("branch", name.as_str()))
        .chain(tags.iter().map(|(_, name, _)| ("tag", name.as_str())))
    {
        findings.extend(scan_name(
            &format!("{kind} {}", masked(name, words)),
            name,
            words,
        ));
    }
    findings.extend(scan_unpushed_changes(repo, words, exempt, skipped)?);
    Ok(findings)
}

pub(super) fn scan_message(place: &str, message: &str, words: &PrivateWords) -> Vec<Finding> {
    scan_text(message, words, true, true)
        .into_iter()
        .map(|(line, rule)| finding(place, line, rule))
        .collect()
}

/// How many characters of a commit hash a finding shows.
pub(super) const SHORT_HASH: usize = 7;

/// A file that a commit adds or changes: the commit, the path and the content (a blob).
pub(super) struct Change {
    hash: String,
    path: String,
    blob: String,
}

/// The files that the commits add or change on any branch or tag and on `HEAD`, without the
/// commits a remote has already: a new branch, a second local branch and a clone are covered, and
/// a repository without remotes yields its whole history.
pub(super) fn unpushed_changes(repo: &Repo, skipped: &str) -> anyhow::Result<Vec<Change>> {
    let raw = repo.git_paths_bytes(&[
        "log",
        "--raw",
        "--no-renames",
        "--no-abbrev",
        "--format=%x1e%H",
        "--branches",
        "--tags",
        "HEAD",
        "--not",
        "--remotes",
    ])?;
    let mut changes = Vec::new();
    let mut hash = String::new();
    for line in String::from_utf8_lossy(&raw).lines() {
        if let Some(short) = line.strip_prefix('\u{1e}') {
            hash = short.chars().take(SHORT_HASH).collect();
            continue;
        }
        // `:old-mode new-mode old-blob new-blob status<TAB>path`
        let Some((fields, path)) = line.strip_prefix(':').and_then(|l| l.split_once('\t')) else {
            continue;
        };
        let fields: Vec<&str> = fields.split_whitespace().collect();
        let (Some(mode), Some(blob), Some(status)) = (fields.get(1), fields.get(3), fields.get(4))
        else {
            continue;
        };
        // A deleted file adds nothing, and a submodule is no file of ours.
        if status.starts_with('D') || *mode == "160000" || path.starts_with(skipped) {
            continue;
        }
        changes.push(Change {
            hash: hash.clone(),
            path: path.to_owned(),
            blob: (*blob).to_owned(),
        });
    }
    Ok(changes)
}

/// The contents of the blobs, by blob, read with one `git cat-file --batch`.
pub(super) fn read_blobs(
    repo: &Repo,
    blobs: &BTreeSet<&str>,
) -> anyhow::Result<BTreeMap<String, Vec<u8>>> {
    let mut found = BTreeMap::new();
    if blobs.is_empty() {
        return Ok(found);
    }
    let input: String = blobs.iter().flat_map(|blob| [*blob, "\n"]).collect();
    let output = repo.git_bytes_with_input(&["cat-file", "--batch"], input.as_bytes())?;
    // Each answer is a line `<blob> <type> <size>`, the content and a line end; `<blob> missing`
    // has no content.
    let mut rest: &[u8] = &output;
    while let Some(end) = rest.iter().position(|byte| *byte == b'\n') {
        let header = String::from_utf8_lossy(&rest[..end]).into_owned();
        rest = &rest[end + 1..];
        let mut parts = header.split(' ');
        let (Some(blob), Some(kind), Some(size)) = (
            parts.next(),
            parts.next(),
            parts.next().and_then(|size| size.parse::<usize>().ok()),
        ) else {
            continue;
        };
        if rest.len() < size {
            break;
        }
        if kind == "blob" {
            found.insert(blob.to_owned(), rest[..size].to_vec());
        }
        rest = rest.get(size + 1..).unwrap_or_default();
    }
    Ok(found)
}

/// The names and contents of the files that the commits not on a remote add or change, through
/// the same reading as the files of the tree. A file that was committed with private data and
/// cleaned or renamed in a later commit is found here.
pub(super) fn scan_unpushed_changes(
    repo: &Repo,
    words: &PrivateWords,
    exempt: &[Exempt],
    skipped: &str,
) -> anyhow::Result<Vec<Finding>> {
    let changes = unpushed_changes(repo, skipped)?;
    let blobs: BTreeSet<&str> = changes.iter().map(|change| change.blob.as_str()).collect();
    let contents = read_blobs(repo, &blobs)?;
    let mut findings = Vec::new();
    let mut named = BTreeSet::new();
    let mut read = BTreeSet::new();
    for change in &changes {
        let place = format!("commit {}, {}", change.hash, masked(&change.path, words));
        if named.insert(&change.path) {
            findings.extend(scan_name(&place, &change.path, words));
        }
        if let (true, Some(bytes)) = (
            read.insert((&change.path, &change.blob)),
            contents.get(&change.blob),
        ) {
            let words_apply = !is_exempt(exempt, &change.path);
            findings.extend(scan_content(&place, bytes, words, words_apply));
        }
    }
    Ok(findings)
}
