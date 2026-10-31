//! Where the answers hold HTML the server rendered: the places the browser client turns a string
//! into `SafeHtml`, and no other.

use std::fmt::Write as _;

use anyhow::Context as _;

use super::super::TS_HEADER;

/// The mark that says a property holds HTML the server rendered.
const SAFE_HTML_MARK: &str = bb_api_words::marks::SAFE_HTML;
/// The step of a path that stands for every item of a list.
const EVERY_ITEM: &str = "*";

/// Where the properties with the mark `x-safe-html` are in the answers: for each operation
/// (`METHOD /path/{template}`), the paths of keys from the top of the answer down to the property,
/// with `*` for every item of a list (M3-F12). The client turns a string into a `SafeHtml` only at
/// these places, so a key of the same name anywhere else is only text.
pub(crate) fn safe_html_paths(
    spec: &str,
) -> anyhow::Result<std::collections::BTreeMap<String, Vec<Vec<String>>>> {
    let spec: serde_json::Value =
        serde_json::from_str(spec).context("the API description is not JSON")?;
    let paths = spec["paths"]
        .as_object()
        .context("the API description has no paths")?;
    let mut found = std::collections::BTreeMap::new();
    for (address, methods) in paths {
        for (method, operation) in methods.as_object().into_iter().flatten() {
            let mut here = Vec::new();
            for (status, answer) in operation["responses"].as_object().into_iter().flatten() {
                let schema = &answer["content"]["application/json"]["schema"];
                if status.starts_with('2') && !schema.is_null() {
                    walk_for_safe_html(&spec, schema, &mut Vec::new(), &mut Vec::new(), &mut here);
                }
            }
            here.sort();
            here.dedup();
            if !here.is_empty() {
                found.insert(format!("{} {address}", method.to_uppercase()), here);
            }
        }
    }
    Ok(found)
}

/// Follows a schema: references, the parts of `allOf`/`oneOf`/`anyOf`, properties and list items.
fn walk_for_safe_html(
    spec: &serde_json::Value,
    schema: &serde_json::Value,
    path: &mut Vec<String>,
    followed: &mut Vec<String>,
    found: &mut Vec<Vec<String>>,
) {
    if let Some(reference) = schema["$ref"].as_str() {
        // A schema that refers to itself is followed once.
        if followed.iter().any(|seen| seen == reference) {
            return;
        }
        let name = reference.rsplit('/').next().unwrap_or_default();
        followed.push(reference.to_owned());
        walk_for_safe_html(
            spec,
            &spec["components"]["schemas"][name],
            path,
            followed,
            found,
        );
        followed.pop();
        return;
    }
    for keyword in ["allOf", "oneOf", "anyOf"] {
        for part in schema[keyword].as_array().into_iter().flatten() {
            walk_for_safe_html(spec, part, path, followed, found);
        }
    }
    for (name, property) in schema["properties"].as_object().into_iter().flatten() {
        path.push(name.clone());
        if property[SAFE_HTML_MARK] == serde_json::json!(true) {
            found.push(path.clone());
        } else {
            walk_for_safe_html(spec, property, path, followed, found);
        }
        path.pop();
    }
    if !schema["items"].is_null() {
        path.push(EVERY_ITEM.to_owned());
        walk_for_safe_html(spec, &schema["items"], path, followed, found);
        path.pop();
    }
}

/// The places from [`safe_html_paths`] as a constant for the browser client.
pub(crate) fn safe_html(spec: &str) -> anyhow::Result<String> {
    let mut rows = String::new();
    for (operation, paths) in safe_html_paths(spec)? {
        let paths: Vec<String> = paths
            .iter()
            .map(|path| {
                let steps: Vec<String> = path.iter().map(|step| format!("{step:?}")).collect();
                format!("[{}]", steps.join(", "))
            })
            .collect();
        let _ = writeln!(rows, "  {operation:?}: [{}],", paths.join(", "));
    }
    Ok(format!(
        "{TS_HEADER}/** Where the answers of each operation hold HTML the server rendered (mark `{SAFE_HTML_MARK}`): the keys from the top of the answer down to the string, `{EVERY_ITEM}` for every item of a list. The client turns the strings there into `SafeHtml`, and no other. */
export const safeHtmlPaths: Readonly<Record<string, readonly (readonly string[])[]>> = {{
{rows}}};
"
    ))
}
