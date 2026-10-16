//! Finding the paths of profiles, the IDs of sessions and the links to pages in a text.

use super::*;

/// The place of the name of an account when the text `rest` (after `Users` or `home`) is a
/// separator run followed by an account that is not a placeholder.
pub(super) fn account_follows(rest: &str, words: &PrivateWords) -> Option<Span> {
    let after_separators = rest.trim_start_matches(SEPARATORS);
    if after_separators.len() == rest.len() {
        return None;
    }
    let skip = rest.len() - after_separators.len();
    let name: String = after_separators
        .chars()
        .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '-'))
        .collect();
    let lower = name.to_lowercase();
    (name.chars().next().is_some_and(char::is_alphanumeric)
        && (!GENERIC_ACCOUNTS.contains(&lower.as_str()) || words.is_own(&lower)))
    .then_some((skip, skip + name.len()))
}

/// The place of the account when the folder at `at` in `line` starts a profile path.
pub(super) fn profile_path_at(
    line: &str,
    at: usize,
    keyword: &str,
    exact: &str,
    words: &PrivateWords,
) -> Option<Span> {
    let before = &line[..at];
    let prefix = before.trim_end_matches(SEPARATORS);
    if prefix.len() == before.len() {
        return None;
    }
    let start = at + keyword.len();
    let (from, to) = account_follows(&line[start..], words)?;
    let drive = prefix
        .strip_suffix(':')
        .and_then(|p| p.chars().next_back())
        .is_some_and(|c| c.is_ascii_alphabetic());
    let mounted = prefix
        .strip_suffix(|c: char| c.is_ascii_alphabetic())
        .is_some_and(|p| p.ends_with('/'));
    let admin_share = prefix.ends_with('$');
    // `\\wsl$\Ubuntu\home\…`, `\\host\share\Users\…`: a network path.
    let network = prefix
        .rsplit(|c: char| c.is_whitespace() || QUOTES.contains(&c))
        .next()
        .is_some_and(|token| token.starts_with("\\\\"));
    let var_home = keyword == "home" && prefix.ends_with("/var");
    let alone = prefix.is_empty()
        || prefix.ends_with(|c: char| !is_word_char(c) && !matches!(c, '/' | '.' | '~'));
    // `C:\Users\…`, `/c/Users/…`, `\\host\c$\Users\…`, `\\wsl$\…\home\…`, `/var/home/…` and
    // `\Users\…` are certain. A bare `/Users/…` or `/home/…` has to be the exact word at the start
    // of a path, not the end of a longer one.
    let certain = drive
        || mounted
        || admin_share
        || network
        || var_home
        || (before.ends_with('\\') && alone)
        || (&line[at..start] == exact && before.ends_with('/') && alone);
    certain.then_some((start + from, start + to))
}

/// The text with the characters of a path that URLs write as `%5C`, `%2F` and `%3A` decoded.
pub(super) fn decode_url_path(line: &str) -> String {
    let mut text = line.to_owned();
    for (code, character) in [("%5c", "\\"), ("%2f", "/"), ("%3a", ":")] {
        while let Some(at) = text.to_ascii_lowercase().find(code) {
            text.replace_range(at..at + code.len(), character);
        }
    }
    text
}

/// Where the accounts of profile paths stand in the text.
pub(super) fn path_spans(text: &str, words: &PrivateWords) -> Vec<Span> {
    let lower = text.to_ascii_lowercase();
    PROFILE_FOLDERS
        .iter()
        .flat_map(|(keyword, exact)| {
            lower
                .match_indices(keyword)
                .filter_map(|(at, _)| profile_path_at(text, at, keyword, exact, words))
        })
        .collect()
}

/// Whether `line` holds a user-profile path with an account name, also when it is URL-encoded.
pub(super) fn has_profile_path(line: &str, words: &PrivateWords) -> bool {
    !path_spans(line, words).is_empty() || !path_spans(&decode_url_path(line), words).is_empty()
}

/// Where the values stand that follow `key` and a colon or an equals sign in `line` (the key in any
/// case, with or without quotes, spaces around the sign).
pub(super) fn value_starts(line: &str, key: &str) -> Vec<usize> {
    let lower = line.to_ascii_lowercase();
    lower
        .match_indices(&key.to_ascii_lowercase())
        .filter_map(|(at, _)| {
            let rest = line[at + key.len()..]
                .trim_start_matches(QUOTES)
                .trim_start()
                .strip_prefix([':', '='])?;
            let value = rest.trim_start().trim_start_matches(QUOTES);
            Some(line.len() - value.len())
        })
        .collect()
}

pub(super) fn id_len(rest: &str) -> usize {
    rest.chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .count()
}

/// Whether the text starts with a UUID.
pub(super) fn starts_with_uuid(text: &str) -> bool {
    let mut groups = text.splitn(UUID_GROUPS.len(), '-');
    UUID_GROUPS.iter().all(|length| {
        groups.next().is_some_and(|group| {
            group.len() >= *length && group[..*length].chars().all(|c| c.is_ascii_hexdigit())
        })
    })
}

/// Where the session IDs stand in the line.
pub(super) fn session_spans(line: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    for start in value_starts(line, MEMORY_SESSION_KEY) {
        let length = id_len(&line[start..]);
        if length >= MIN_ID_LEN {
            spans.push((start, start + length));
        }
    }
    for key in UUID_SESSION_KEYS {
        for start in value_starts(line, key) {
            if starts_with_uuid(&line[start..]) {
                spans.push((start, start + id_len(&line[start..])));
            }
        }
    }
    spans.extend(id_spans_after(line, SESSION_LINK));
    let lower = line.to_ascii_lowercase();
    for (at, _) in lower.match_indices(SESSION_PREFIX) {
        let start = at + SESSION_PREFIX.len();
        let run: String = line[start..]
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect();
        if run.len() >= LONG_ID_LEN
            && run.chars().any(|c| c.is_ascii_digit())
            && run.chars().any(|c| c.is_ascii_alphabetic())
        {
            spans.push((start, start + run.len()));
        }
    }
    spans
}

/// Where the IDs stand that follow `prefix` (in lower case) in the line, in any case.
pub(super) fn id_spans_after(line: &str, prefix: &str) -> Vec<Span> {
    let lower = line.to_ascii_lowercase();
    lower
        .match_indices(prefix)
        .filter_map(|(at, _)| {
            let start = at + prefix.len();
            let length = id_len(&line[start..]);
            (length >= MIN_ID_LEN).then_some((start, start + length))
        })
        .collect()
}

/// Where the IDs of published pages stand in the line.
pub(super) fn page_link_spans(line: &str) -> Vec<Span> {
    PAGE_LINKS
        .iter()
        .flat_map(|link| id_spans_after(line, link))
        .collect()
}

/// Whether the line is a session trailer (`Claude-Session` and a colon, spaces allowed between;
/// a comment mark in front does not hide it, whatever character Git uses for comments).
pub(super) fn is_session_trailer(line: &str) -> bool {
    line.trim_start_matches(|c: char| !c.is_alphanumeric())
        .to_ascii_lowercase()
        .strip_prefix(COMMIT_TRAILER)
        .is_some_and(|rest| rest.trim_start().starts_with(':'))
}

/// The rules one line breaks. `in_message` adds the rules of commit messages; `names` says
/// whether the rule on private words applies.
pub(super) fn rules_of(
    line: &str,
    words: &PrivateWords,
    in_message: bool,
    names: bool,
) -> Vec<Rule> {
    let mut rules = Vec::new();
    if has_profile_path(line, words) {
        rules.push(Rule::ProfilePath);
    }
    if !session_spans(line).is_empty() {
        rules.push(Rule::SessionId);
    }
    if in_message && is_session_trailer(line) {
        rules.push(Rule::SessionTrailer);
    }
    if !page_link_spans(line).is_empty() {
        rules.push(Rule::PageLink);
    }
    if names && !words.word_spans(line).is_empty() {
        rules.push(Rule::PrivateWord);
    }
    rules
}

/// `(line number, rule)` for every rule broken in `text`.
pub(super) fn scan_text(
    text: &str,
    words: &PrivateWords,
    in_message: bool,
    names: bool,
) -> Vec<(usize, Rule)> {
    text.lines()
        .enumerate()
        .flat_map(|(index, line)| {
            rules_of(line, words, in_message, names)
                .into_iter()
                .map(move |rule| (index + 1, rule))
        })
        .collect()
}

/// A name (of a file, a branch or a tag) as a finding may show it: the private words, accounts
/// and IDs in it replaced by [`MASK`].
pub(super) fn masked(name: &str, words: &PrivateWords) -> String {
    let decoded = decode_url_path(name);
    let text = if path_spans(name, words).is_empty() && !path_spans(&decoded, words).is_empty() {
        decoded.as_str()
    } else {
        name
    };
    let mut spans = words.word_spans(text);
    spans.extend(session_spans(text));
    spans.extend(page_link_spans(text));
    spans.extend(path_spans(text, words));
    spans.sort_unstable();
    let mut shown = String::new();
    let mut done = 0;
    for (from, to) in spans {
        if to <= done {
            continue;
        }
        shown.push_str(&text[done..from.max(done)]);
        shown.push_str(MASK);
        done = to;
    }
    shown.push_str(&text[done..]);
    shown
}
