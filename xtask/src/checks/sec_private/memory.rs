//! The memory notes of Claude Code (`.claude/memory/*.md`): the keys it writes into the front
//! matter of every note it saves must not reach Git.
//!
//! The session key holds the ID of a session and is refused by the check and removed after an
//! edit. The keys for the time of the last change and for the kind of node are removed after an
//! edit as well, also when they stand indented below `metadata`.

use std::fs;
use std::path::Path;

use super::{CLAUDE_FOLDER, MEMORY_FOLDER, MEMORY_SESSION_KEY, QUOTES};

/// The other keys Claude Code adds to the front matter.
const GENERATED_KEYS: [&str; 2] = ["modified", "node_type"];
/// The key that groups others in the front matter.
const GROUP_KEY: &str = "metadata";
/// The line that opens and closes the front matter.
const FRONT_MATTER_FENCE: &str = "---";

/// Whether `path` is a memory note of Claude Code (`.claude/memory/*.md`).
pub(super) fn is_memory_note(path: &Path) -> bool {
    let folder = path.parent();
    path.extension().is_some_and(|ending| ending == "md")
        && folder
            .and_then(Path::file_name)
            .is_some_and(|name| name == MEMORY_FOLDER)
        && folder
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .is_some_and(|name| name == CLAUDE_FOLDER)
}

/// The key a line sets (`key: value` or `key = value`, with or without quotes, indented or not),
/// and what follows the sign.
fn key_and_value(line: &str) -> Option<(&str, &str)> {
    let start = line.trim_start().trim_start_matches(QUOTES);
    let length = start
        .find(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '-')))
        .unwrap_or(start.len());
    let (key, rest) = start.split_at(length);
    let value = rest
        .trim_start_matches(QUOTES)
        .trim_start()
        .strip_prefix([':', '='])?;
    (!key.is_empty()).then_some((key, value.trim()))
}

fn is_key(line: &str, wanted: &str) -> bool {
    key_and_value(line).is_some_and(|(key, _)| key.eq_ignore_ascii_case(wanted))
}

/// The numbers of the lines (from 1) that set the session key, whatever the value.
pub(super) fn session_key_lines(text: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| is_key(line, MEMORY_SESSION_KEY))
        .map(|(index, _)| index + 1)
        .collect()
}

/// The text of a note without the keys Claude Code adds. The session key goes wherever it
/// stands; the others only in the front matter.
fn without_generated_keys(text: &str) -> String {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut keep = vec![true; lines.len()];
    let mut in_front = false;
    let mut front_ended = false;
    let mut front = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let bare = line.trim_end();
        if bare == FRONT_MATTER_FENCE && index == 0 {
            in_front = true;
            continue;
        }
        if bare == FRONT_MATTER_FENCE && in_front && !front_ended {
            in_front = false;
            front_ended = true;
            continue;
        }
        if in_front {
            front.push(index);
        }
        let generated = in_front && GENERATED_KEYS.iter().any(|key| is_key(line, key));
        if generated || is_key(line, MEMORY_SESSION_KEY) {
            keep[index] = false;
        }
    }
    // A group with nothing left below it goes as well.
    for (at, &index) in front.iter().enumerate() {
        let empty_group = keep[index]
            && key_and_value(lines[index]).is_some_and(|(key, value)| {
                key.eq_ignore_ascii_case(GROUP_KEY) && value.is_empty()
            });
        let next = front[at + 1..].iter().find(|&&later| keep[later]);
        let has_children = next.is_some_and(|&later| lines[later].starts_with([' ', '\t']));
        if empty_group && !has_children {
            keep[index] = false;
        }
    }
    lines
        .iter()
        .zip(keep)
        .filter_map(|(line, kept)| kept.then_some(*line))
        .collect()
}

/// Removes the keys Claude Code adds from a memory note after an edit. Returns whether it changed
/// the file.
pub(crate) fn strip_generated_keys(path: &Path) -> anyhow::Result<bool> {
    if !is_memory_note(path) || !path.is_file() {
        return Ok(false);
    }
    let text = fs::read_to_string(path)?;
    let kept = without_generated_keys(&text);
    if kept == text {
        return Ok(false);
    }
    fs::write(path, kept)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keys_are_removed_from_the_front_matter_also_when_indented() {
        let key = MEMORY_SESSION_KEY;
        let note = format!(
            "---\nname: n\n{key}: 0123456789abcdef\nmodified: 2026-01-01T00:00:00Z\nnode_type: memory\nmetadata:\n  node_type: memory\n  modified: 2026\n---\r\nBody\nmodified: stays in the body\n"
        );
        assert_eq!(
            without_generated_keys(&note),
            "---\nname: n\n---\r\nBody\nmodified: stays in the body\n"
        );
    }

    #[test]
    fn a_group_with_other_children_stays() {
        let note = "---\nmetadata:\n  node_type: memory\n  owner: someone\n---\n";
        assert_eq!(
            without_generated_keys(note),
            "---\nmetadata:\n  owner: someone\n---\n"
        );
    }

    #[test]
    fn the_session_key_is_found_with_or_without_a_value() {
        let key = MEMORY_SESSION_KEY;
        let text = format!(
            "a\n{key}:\n  \"{}\" : x\nnot {key}: x\n",
            key.to_lowercase()
        );
        assert_eq!(session_key_lines(&text), [2, 3]);
    }
}
