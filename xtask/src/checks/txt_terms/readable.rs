//! What people read, cut out of the places that hold it (TXT-TERMS): the words of a message
//! without its placeholders and keywords, the prose of a Markdown page without its code, and the
//! names in the API description.

use serde_json::Value;

use crate::checks::api_spec;
use crate::generate::api::SPEC_PATH;

/// A message without what is code: the names of placeholders (`{path}`) and the keywords of
/// `{count, plural, one {…} other {…}}` and `{kind, select, a {…} b {…}}` (the selectors `one`,
/// `other`, `=0`, `a`, `b`). The text around them and the text inside every option stay.
pub(super) fn message_text(message: &str) -> String {
    let chars: Vec<char> = message.chars().collect();
    let mut text = String::with_capacity(message.len());
    let mut at = 0;
    while at < chars.len() {
        if chars[at] == '{'
            && let Some(end) = closing(&chars, at)
        {
            argument(&chars[at + 1..end], &mut text);
            at = end + 1;
        } else {
            text.push(chars[at]);
            at += 1;
        }
    }
    text
}

/// The place of the `}` that closes the `{` at `open`.
fn closing(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0_u32;
    for (offset, c) in chars[open..].iter().enumerate() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// What the inside of one `{…}` adds to the text: nothing for a placeholder, the text of every
/// option for a plural or a select.
fn argument(inside: &[char], text: &mut String) {
    text.push(' ');
    let inside: String = inside.iter().collect();
    let mut parts = inside.splitn(3, ',');
    let (_, Some(kind), Some(options)) = (parts.next(), parts.next(), parts.next()) else {
        return;
    };
    if !matches!(kind.trim(), "plural" | "select" | "selectordinal") {
        return;
    }
    let chars: Vec<char> = options.chars().collect();
    let mut at = 0;
    while at < chars.len() {
        // The selector is skipped up to the `{` of its text.
        let Some(open) = chars[at..].iter().position(|c| *c == '{').map(|p| at + p) else {
            break;
        };
        let Some(end) = closing(&chars, open) else {
            break;
        };
        let option: String = chars[open + 1..end].iter().collect();
        text.push_str(&message_text(&option));
        text.push(' ');
        at = end + 1;
    }
}

/// A Markdown page without what is not prose: fenced code (with backticks or tildes), indented code, code spans
/// and the addresses of links. Blank lines take the place of what is left out, so the lines keep
/// their numbers.
pub(super) fn prose(page: &str) -> String {
    let mut text = String::with_capacity(page.len());
    let mut fence: Option<&str> = None;
    let mut previous_blank = true;
    let mut in_list = false;
    let mut in_indented = false;
    for line in page.lines() {
        let trimmed = line.trim_start();
        if let Some(marker) = fence {
            if trimmed.starts_with(marker) {
                fence = None;
            }
            text.push('\n');
            continue;
        }
        if let Some(marker) = ["```", "~~~"]
            .into_iter()
            .find(|marker| trimmed.starts_with(marker))
        {
            fence = Some(marker);
            text.push('\n');
            continue;
        }
        let blank = trimmed.is_empty();
        let indented = line.starts_with("    ") || line.starts_with('\t');
        // Four spaces start code after a blank line, but not inside a list, where they continue
        // an item.
        if indented && !blank && !in_list && (previous_blank || in_indented) {
            in_indented = true;
            text.push('\n');
            previous_blank = false;
            continue;
        }
        in_indented = false;
        if !blank && !indented {
            in_list = is_list_item(trimmed);
        }
        previous_blank = blank;
        text.push_str(&without_spans(line));
        text.push('\n');
    }
    text
}

/// Whether the line starts an item of a list: `- `, `* `, `+ ` or `1. `.
fn is_list_item(line: &str) -> bool {
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    ["- ", "* ", "+ "]
        .iter()
        .any(|marker| line.starts_with(marker))
        || (digits > 0 && line[digits..].starts_with(". "))
}

/// A line without `code spans` and without the address of a link: `[text](address)`.
fn without_spans(line: &str) -> String {
    let mut text = String::with_capacity(line.len());
    let mut in_span = false;
    let mut in_address = false;
    let mut previous = ' ';
    for c in line.chars() {
        match c {
            '`' => in_span = !in_span,
            '(' if previous == ']' && !in_span => in_address = true,
            ')' if in_address => in_address = false,
            _ if in_span || in_address => {}
            _ => text.push(c),
        }
        previous = c;
    }
    text
}

/// One name in the API description.
pub(super) struct ApiName {
    /// What kind of name: `path`, `schema`, `property`, `enum`, `operation`, `parameter`.
    pub(super) kind: &'static str,
    /// The name with what it belongs to, for the place (`Schema.property`).
    pub(super) place: String,
    /// The text that is read as words.
    pub(super) text: String,
}

/// The names in the API description: paths, the names in a path (`{id}`), schema names, property
/// names, the values of enums, operation IDs and the names of parameters.
pub(super) fn api_names(spec: &Value) -> Vec<ApiName> {
    let mut names = Vec::new();
    let mut add = |kind: &'static str, place: String, text: &str| {
        names.push(ApiName {
            kind,
            place,
            text: text.to_owned(),
        });
    };
    for (path, _) in api_spec::paths(spec).into_iter().flatten() {
        add("path", path.to_owned(), path);
    }
    for operation in api_spec::operations(spec).into_iter().flatten() {
        if let Some(id) = operation.text("operationId") {
            add("operation", id.to_owned(), id);
        }
        parameters(operation.body, &mut |name| {
            add("parameter", name.to_owned(), name);
        });
    }
    for parameter in api_spec::shared_parameters(spec) {
        if let Some(name) = parameter.get("name").and_then(Value::as_str) {
            add("parameter", name.to_owned(), name);
        }
    }
    for (name, schema) in api_spec::schemas(spec) {
        add("schema", name.to_owned(), name);
        walk_schema(name, schema, &mut add);
    }
    names
}

/// The names of the parameters of an operation.
fn parameters(operation: &Value, add: &mut dyn FnMut(&str)) {
    for parameter in operation
        .get("parameters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(name) = parameter.get("name").and_then(Value::as_str) {
            add(name);
        }
    }
}

/// The property names and enum values anywhere in a schema.
fn walk_schema(schema_name: &str, value: &Value, add: &mut dyn FnMut(&'static str, String, &str)) {
    if let Some(properties) = value.get("properties").and_then(Value::as_object) {
        for (property, inner) in properties {
            add("property", format!("{schema_name}.{property}"), property);
            walk_schema(schema_name, inner, add);
        }
    }
    for word in value
        .get("enum")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        add("enum", format!("{schema_name} {word}"), word);
    }
    for key in ["items", "additionalProperties"] {
        if let Some(inner) = value.get(key) {
            walk_schema(schema_name, inner, add);
        }
    }
    for key in ["oneOf", "anyOf", "allOf"] {
        for inner in value
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            walk_schema(schema_name, inner, add);
        }
    }
}

/// What a finding says about where an API name is.
pub(super) fn api_place(name: &ApiName) -> (String, String) {
    (
        format!("api:{} {}", name.kind, name.place),
        format!("{SPEC_PATH}: {} {}", name.kind, name.place),
    )
}
