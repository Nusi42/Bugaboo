//! The tidying of the finished API description (WP-16, API changes A10, B1 and B12).
//!
//! utoipa writes each route's answers, headers and paging parameters out in full, and the pages of a
//! list with a copy of the item. The description is smaller and says each thing once when it is
//! tidied at the end, in one place for every route:
//!
//! - a page of a list refers to its item (`Page_ProjectItem` → `ProjectItem`);
//! - the answers that many routes share, the `ETag` and `Retry-After` headers and the paging
//!   parameters `fields`, `cursor` and `limit` are written once in `components` and referred to;
//! - the texts people read are free of the links of the Rust documentation and of the numbers of
//!   work packages and decision records.

use super::ordered::Ordered;

/// The methods an operation can have.
const METHODS: [&str; 8] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

/// How many routes must share an answer for it to be written once in `components`.
const SHARED_BY: usize = 3;

/// The paging parameters, by the name of the parameter and the name it has in `components`.
const PAGING_PARAMETERS: [(&str, &str); 3] = [
    ("fields", "Fields"),
    ("cursor", "Cursor"),
    ("limit", "Limit"),
];

/// The headers, by the name of the header and the name it has in `components`.
const HEADERS: [(&str, &str); 2] = [("etag", "ETag"), ("retry-after", "RetryAfter")];

/// The words for the status of an answer, as the name of a shared answer.
fn status_name(status: &str) -> String {
    match status {
        "304" => "NotModified",
        "400" => "BadRequest",
        "401" => "NotSignedIn",
        "403" => "Forbidden",
        "404" => "NotFound",
        "409" => "Conflict",
        "413" => "TooLarge",
        "429" => "TooManyRequests",
        "500" => "InternalError",
        "502" => "BadGateway",
        "503" => "Unavailable",
        other => return format!("Answer{other}"),
    }
    .to_owned()
}

/// Tidies the finished description.
pub(super) fn tidy(document: &mut Ordered) {
    pages_refer_to_their_items(document);
    share_headers(document);
    share_paging_parameters(document);
    share_answers(document);
    clean_texts(document);
}

/// A page of a list holds a copy of its item in the description that utoipa writes; it refers to
/// the schema of the item instead, so the item is written once and no schema stands alone.
fn pages_refer_to_their_items(document: &mut Ordered) {
    let Some(schemas) = document
        .get_mut("components")
        .and_then(|components| components.get_mut("schemas"))
    else {
        return;
    };
    let names = schemas.keys();
    for name in &names {
        let Some(item) = name.strip_prefix("Page_") else {
            continue;
        };
        if !names.iter().any(|known| known == item) {
            continue;
        }
        let slot = schemas
            .get_mut(name)
            .and_then(|page| page.get_mut("properties"))
            .and_then(|properties| properties.get_mut("items"))
            .and_then(|items| items.get_mut("items"));
        if let Some(slot) = slot
            && !slot.is_reference()
        {
            *slot = Ordered::reference(&format!("#/components/schemas/{item}"));
        }
    }
}

/// Every operation of the description, to change.
fn for_each_operation(document: &mut Ordered, visit: &mut impl FnMut(&mut Ordered)) {
    let Some(paths) = document.get_mut("paths") else {
        return;
    };
    for path in paths.keys() {
        let Some(item) = paths.get_mut(&path) else {
            continue;
        };
        for method in METHODS {
            if let Some(operation) = item.get_mut(method) {
                visit(operation);
            }
        }
    }
}

/// A part of `components`, made when it is not there.
fn component_part<'a>(document: &'a mut Ordered, part: &str) -> Option<&'a mut Ordered> {
    if document.get("components").is_none() {
        document.set("components", Ordered::empty_object());
    }
    let components = document.get_mut("components")?;
    if components.get(part).is_none() {
        components.set(part, Ordered::empty_object());
    }
    components.get_mut(part)
}

/// A name for `wanted` that `part` does not use yet.
fn free_name(part: &Ordered, wanted: &str) -> String {
    let taken = part.keys();
    if !taken.iter().any(|name| name == wanted) {
        return wanted.to_owned();
    }
    let mut number = 2_u32;
    loop {
        let name = format!("{wanted}{number}");
        if !taken.contains(&name) {
            return name;
        }
        number = number.saturating_add(1);
    }
}

/// The headers of the answers (`ETag`, `Retry-After`) are written once.
fn share_headers(document: &mut Ordered) {
    let mut found: Vec<(&str, &str, Ordered)> = Vec::new();
    for_each_operation(document, &mut |operation| {
        let Some(responses) = operation.get_mut("responses") else {
            return;
        };
        for status in responses.keys() {
            let Some(headers) = responses
                .get_mut(&status)
                .and_then(|response| response.get_mut("headers"))
            else {
                continue;
            };
            for (header, name) in HEADERS {
                let Some(slot) = headers.get_mut(header) else {
                    continue;
                };
                if slot.is_reference() {
                    continue;
                }
                let written = slot.written();
                let known = found
                    .iter()
                    .any(|(known, _, value)| *known == header && value.written() == written);
                let moved = std::mem::replace(
                    slot,
                    Ordered::reference(&format!("#/components/headers/{name}")),
                );
                if !known {
                    found.push((header, name, moved));
                }
            }
        }
    });
    if found.is_empty() {
        return;
    }
    if let Some(part) = component_part(document, "headers") {
        for (_, name, value) in found {
            part.set(name, value);
        }
    }
}

/// The paging parameters are written once, when every route writes one of them the same way.
fn share_paging_parameters(document: &mut Ordered) {
    let mut variants: Vec<(&str, Vec<String>)> = PAGING_PARAMETERS
        .iter()
        .map(|(parameter, _)| (*parameter, Vec::new()))
        .collect();
    let mut seen = Vec::new();
    for_each_operation(document, &mut |operation| {
        let Some(Ordered::Array(parameters)) = operation.get_mut("parameters") else {
            return;
        };
        for parameter in parameters.iter() {
            let name = parameter.get("name").and_then(Ordered::text);
            let location = parameter.get("in").and_then(Ordered::text);
            if let (Some(name), Some("query")) = (name, location)
                && let Some((_, written)) = variants.iter_mut().find(|(known, _)| *known == name)
            {
                let text = parameter.written();
                if !written.contains(&text) {
                    written.push(text);
                }
            }
        }
        seen.push(());
    });
    let shared: Vec<(&str, &str)> = PAGING_PARAMETERS
        .iter()
        .filter(|(parameter, _)| {
            variants
                .iter()
                .any(|(known, written)| known == parameter && written.len() == 1)
        })
        .copied()
        .collect();
    let mut definitions: Vec<(&str, Ordered)> = Vec::new();
    for_each_operation(document, &mut |operation| {
        let Some(Ordered::Array(parameters)) = operation.get_mut("parameters") else {
            return;
        };
        for parameter in parameters.iter_mut() {
            let name = parameter
                .get("name")
                .and_then(Ordered::text)
                .map(str::to_owned);
            let in_query = parameter.get("in").and_then(Ordered::text) == Some("query");
            let Some((_, component)) = shared
                .iter()
                .find(|(known, _)| in_query && Some(*known) == name.as_deref())
            else {
                continue;
            };
            let moved = std::mem::replace(
                parameter,
                Ordered::reference(&format!("#/components/parameters/{component}")),
            );
            if !definitions.iter().any(|(known, _)| known == component) {
                definitions.push((component, moved));
            }
        }
    });
    if definitions.is_empty() {
        return;
    }
    if let Some(part) = component_part(document, "parameters") {
        for (name, value) in definitions {
            part.set(name, value);
        }
    }
}

/// The answers of a refusal or of a match that [`SHARED_BY`] routes or more write the same way are
/// written once (an answer of success is the answer of its route). The most
/// usual one of a status is named for the status (`NotFound`), the others get a number.
fn share_answers(document: &mut Ordered) {
    // How often each answer is written, by status.
    let mut counts: Vec<(String, String, usize)> = Vec::new();
    for_each_operation(document, &mut |operation| {
        let Some(responses) = operation.get("responses") else {
            return;
        };
        for status in responses.keys() {
            let Some(response) = responses.get(&status) else {
                continue;
            };
            if response.is_reference() || status.starts_with('2') {
                continue;
            }
            let written = response.written();
            match counts
                .iter_mut()
                .find(|(known, text, _)| *known == status && *text == written)
            {
                Some((_, _, count)) => *count += 1,
                None => counts.push((status, written, 1)),
            }
        }
    });
    let mut shared: Vec<(String, String, usize)> = counts
        .into_iter()
        .filter(|(_, _, count)| *count >= SHARED_BY)
        .collect();
    shared.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
    if shared.is_empty() {
        return;
    }
    // Their names, in the order of use.
    let mut names: Vec<(String, String, String)> = Vec::new();
    for (status, written, _) in &shared {
        let wanted = status_name(status);
        let mut name = wanted.clone();
        let mut number = 1;
        while names.iter().any(|(_, _, known)| *known == name) {
            number += 1;
            name = format!("{wanted}{number}");
        }
        names.push((status.clone(), written.clone(), name));
    }
    let mut definitions: Vec<(String, Ordered)> = Vec::new();
    for_each_operation(document, &mut |operation| {
        let Some(responses) = operation.get_mut("responses") else {
            return;
        };
        for status in responses.keys() {
            let Some(response) = responses.get_mut(&status) else {
                continue;
            };
            if response.is_reference() || status.starts_with('2') {
                continue;
            }
            let written = response.written();
            let Some((_, _, name)) = names
                .iter()
                .find(|(known, text, _)| *known == status && *text == written)
            else {
                continue;
            };
            let moved = std::mem::replace(
                response,
                Ordered::reference(&format!("#/components/responses/{name}")),
            );
            if !definitions.iter().any(|(known, _)| known == name) {
                definitions.push((name.clone(), moved));
            }
        }
    });
    if let Some(part) = component_part(document, "responses") {
        for (name, value) in definitions {
            let name = free_name(part, &name);
            part.set(&name, value);
        }
    }
}

/// The texts people read have no link of the Rust documentation (`[`Page`]`) and no number of a
/// work package or a decision record (`(ADR-0027)`).
fn clean_texts(document: &mut Ordered) {
    document.for_each_mut(&mut |value| {
        if let Ordered::Object(entries) = value {
            for (key, text) in entries {
                if (key == "description" || key == "summary")
                    && let Ordered::Leaf(serde_json::Value::String(words)) = text
                {
                    *words = clean_text(words);
                }
            }
        }
    });
}

/// The prefixes of the numbers of the work: a feature, a decision record, a follow-up.
const TICKET_PREFIXES: [&str; 22] = [
    "ADR", "SEC", "ISS", "WF", "ATT", "NTF", "CF", "TAG", "PERF", "PRIV", "CFG", "AUTH", "UX",
    "WP", "COMP", "OPS", "PROJ", "MCP", "ROLE", "ORG", "SET", "GIT",
];

/// Whether `word` is the number of a work item (`ADR-0027`, `M3-F12`, `ISS-02`).
fn is_ticket(word: &str) -> bool {
    let Some((prefix, number)) = word.split_once('-') else {
        return false;
    };
    let number = number.strip_prefix(['F', 'Q']).unwrap_or(number);
    let milestone = prefix.len() >= 2
        && prefix.starts_with('M')
        && prefix[1..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit())
        && prefix[1..]
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase());
    (milestone || TICKET_PREFIXES.contains(&prefix))
        && !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit())
}

/// The text without the links of the Rust documentation and without the group in brackets that
/// holds only numbers of work items.
fn clean_text(text: &str) -> String {
    let linked = unlink(text);
    let mut cleaned = String::with_capacity(linked.len());
    let mut rest = linked.as_str();
    while let Some(open) = rest.find('(') {
        let (before, from_open) = rest.split_at(open);
        let Some(close) = from_open.find(')') else {
            break;
        };
        let inside = &from_open[1..close];
        let only_tickets =
            !inside.is_empty() && inside.split([',', ';']).map(str::trim).all(is_ticket);
        if only_tickets {
            cleaned.push_str(before.trim_end_matches(' '));
            rest = &from_open[close + 1..];
        } else {
            cleaned.push_str(before);
            cleaned.push('(');
            rest = &from_open[1..];
        }
    }
    cleaned.push_str(rest);
    cleaned
}

/// A link of the Rust documentation (`[` then a name in backticks then `]`) becomes the name in
/// backticks.
fn unlink(text: &str) -> String {
    text.replace("[`", "`").replace("`]", "`")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_number_of_the_work_and_a_link_of_the_documentation_do_not_reach_a_reader() {
        assert_eq!(
            clean_text("The levels that count (ADR-0027). Without it all."),
            "The levels that count. Without it all."
        );
        assert_eq!(
            clean_text("Kept as a draft (M3-F176, ISS-02)."),
            "Kept as a draft."
        );
        assert_eq!(
            clean_text("Cut to the chosen fields ([`Page::of`])."),
            "Cut to the chosen fields (`Page::of`)."
        );
        // An example is not a number of the work.
        assert_eq!(
            clean_text("The readable ID (WEB-142) of the issue."),
            "The readable ID (WEB-142) of the issue."
        );
        assert_eq!(
            clean_text("Two things (ADR-0025, decision 6)."),
            "Two things (ADR-0025, decision 6)."
        );
    }

    #[test]
    fn the_tidying_writes_a_shared_answer_once_and_refers_to_it() {
        let text = r#"{
          "paths": {
            "/a": {"get": {"parameters": [{"name": "limit", "in": "query"}],
              "responses": {"200": {"headers": {"etag": {"description": "tag"}}}, "404": {"description": "none"}}}},
            "/b": {"get": {"parameters": [{"name": "limit", "in": "query"}],
              "responses": {"200": {"headers": {"etag": {"description": "tag"}}}, "404": {"description": "none"}}}},
            "/c": {"get": {"responses": {"404": {"description": "none"}}}}
          },
          "components": {"schemas": {"Item": {"type": "object"}, "Page_Item": {"properties": {"items": {"type": "array", "items": {"type": "object"}}}}}}
        }"#;
        let tidy_text = super::super::ordered::finished_text(text).unwrap();
        let tidied: serde_json::Value = serde_json::from_str(&tidy_text).unwrap();
        assert_eq!(
            tidied["paths"]["/c"]["get"]["responses"]["404"]["$ref"],
            "#/components/responses/NotFound"
        );
        assert_eq!(
            tidied["components"]["responses"]["NotFound"]["description"],
            "none"
        );
        assert_eq!(
            tidied["paths"]["/a"]["get"]["responses"]["200"]["headers"]["etag"]["$ref"],
            "#/components/headers/ETag"
        );
        assert_eq!(
            tidied["components"]["headers"]["ETag"]["description"],
            "tag"
        );
        assert_eq!(
            tidied["paths"]["/b"]["get"]["parameters"][0]["$ref"],
            "#/components/parameters/Limit"
        );
        assert_eq!(
            tidied["components"]["schemas"]["Page_Item"]["properties"]["items"]["items"]["$ref"],
            "#/components/schemas/Item"
        );
    }
}
