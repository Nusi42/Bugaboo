//! The comparison of two API descriptions, node by node.
//!
//! The frozen description and the new one are walked in parallel: every operation, parameter,
//! request body and answer, and from there through each `$ref` into the component schemas, down
//! every `properties`, `items`, `additionalProperties`, `oneOf`, `anyOf` and `allOf` branch. A node
//! is named by its path (`POST /api/v1/issues requestBody application/json > variant(kind=move) >
//! reason`), and the keywords of the frozen node are compared with those of the new one by the
//! *side* the node is on:
//!
//! - **request** (the client sends it): it may only loosen. Nothing becomes required, nothing new
//!   is required inside what was there, no type, format, `enum`, `pattern`, `default`, limit or
//!   `additionalProperties: false` is added or tightened, no variant goes.
//! - **answer** (the client reads it): it may only grow. No member, variant, value, header, media
//!   type or answer goes, nothing stops being always there, `null` is not added, no type is widened,
//!   no format changes.
//!
//! A schema that both sides reach is walked once for each side. Answer enums, `kind` unions, variants
//! and error codes are open, so what the new description adds there is no finding. What is new has
//! no frozen node to be compared with, so a new member brings its own rules.

use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

use crate::checks::api_spec::{self, resolve};

/// Which side of the API a node is on.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Side {
    /// The client sends it.
    Request,
    /// The client reads it.
    Answer,
    /// No operation reaches it; only what is there must stay.
    Neither,
}

/// A schema as one comparison sees it: `$ref` followed, `allOf` folded in, `null` taken out of the
/// alternatives.
struct Shape {
    /// The name of the schema the node refers to.
    reference: Option<String>,
    /// The keywords, without `oneOf` and `anyOf`.
    body: Value,
    /// Whether `null` is a value.
    nullable: bool,
    /// The alternatives of `oneOf` or `anyOf`, as written; empty when the node has none.
    members: Vec<Value>,
}

/// The name of the schema a `$ref` points to.
fn ref_name(reference: &str) -> &str {
    reference.rsplit('/').next().unwrap_or(reference)
}

/// Puts what `from` says into `into`, where `into` says nothing: a keyword of its own wins,
/// `properties` and `required` are joined.
fn merge(into: &mut Value, from: Value) {
    let (Some(into), Value::Object(from)) = (into.as_object_mut(), from) else {
        return;
    };
    for (key, value) in from {
        match (key.as_str(), into.get_mut(&key)) {
            ("properties", Some(Value::Object(own))) => {
                if let Value::Object(more) = value {
                    for (name, property) in more {
                        own.entry(name).or_insert(property);
                    }
                }
            }
            ("required", Some(Value::Array(own))) => {
                if let Value::Array(more) = value {
                    for name in more {
                        if !own.contains(&name) {
                            own.push(name);
                        }
                    }
                }
            }
            (_, Some(_)) => {}
            (_, None) => {
                into.insert(key, value);
            }
        }
    }
}

/// A schema with its `$ref` followed and its `allOf` folded in, and the name it was referred to by.
fn flatten(spec: &Value, node: &Value, depth: usize) -> (Option<String>, Value) {
    if depth > 24 {
        return (None, node.clone());
    }
    let mut reference = None;
    let mut current = node.clone();
    if let Some(target) = node.get("$ref").and_then(Value::as_str) {
        reference = Some(ref_name(target).to_owned());
        if let Some(found) = target.strip_prefix('#').and_then(|p| spec.pointer(p)) {
            current = flatten(spec, found, depth + 1).1;
        }
    }
    let parts = current
        .as_object_mut()
        .and_then(|members| members.remove("allOf"))
        .and_then(|parts| parts.as_array().cloned())
        .unwrap_or_default();
    let single = parts.len() == 1;
    for part in &parts {
        let (inner_reference, inner) = flatten(spec, part, depth + 1);
        if single && reference.is_none() {
            reference = inner_reference;
        }
        merge(&mut current, inner);
    }
    (reference, current)
}

/// Whether a node says "nothing" as a value: `{"type": "null"}`.
fn is_null(spec: &Value, node: &Value) -> bool {
    let body = flatten(spec, node, 0).1;
    body.get("type").and_then(Value::as_str) == Some("null")
}

fn shape(spec: &Value, node: &Value) -> Shape {
    let (reference, mut body) = flatten(spec, node, 0);
    let mut nullable = body.get("nullable") == Some(&Value::Bool(true));
    match body.get("type") {
        Some(Value::Array(kinds)) => nullable |= kinds.iter().any(|kind| kind == "null"),
        Some(Value::String(kind)) => nullable |= kind == "null",
        _ => {}
    }
    let mut members = Vec::new();
    for key in ["oneOf", "anyOf"] {
        let listed = body
            .as_object_mut()
            .and_then(|object| object.remove(key))
            .and_then(|list| list.as_array().cloned())
            .unwrap_or_default();
        for member in listed {
            if is_null(spec, &member) {
                nullable = true;
            } else {
                members.push(member);
            }
        }
    }
    if members.len() == 1 {
        // One alternative besides `null` is that alternative.
        let inner = shape(spec, &members[0]);
        let mut joined = inner.body;
        merge(&mut joined, body);
        return Shape {
            reference: inner.reference.or(reference),
            body: joined,
            nullable: nullable || inner.nullable,
            members: inner.members,
        };
    }
    Shape {
        reference,
        body,
        nullable,
        members,
    }
}

/// The types a node has, `null` left out; an object with `properties` is an object, an array with
/// `items` an array.
fn kinds(body: &Value) -> BTreeSet<String> {
    let mut found: BTreeSet<String> = match body.get("type") {
        Some(Value::String(kind)) => BTreeSet::from([kind.clone()]),
        Some(Value::Array(kinds)) => kinds
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => BTreeSet::new(),
    };
    found.remove("null");
    if found.is_empty() {
        if body.get("properties").is_some() {
            found.insert("object".to_owned());
        } else if body.get("items").is_some() {
            found.insert("array".to_owned());
        }
    }
    found
}

/// Whether every type of `inner` is one of `outer` (an integer is a number).
fn fits(inner: &BTreeSet<String>, outer: &BTreeSet<String>) -> bool {
    inner
        .iter()
        .all(|kind| outer.contains(kind) || (kind == "integer" && outer.contains("number")))
}

/// The values an `enum` or a `const` lists.
fn values(body: &Value) -> Option<Vec<Value>> {
    match (body.get("enum"), body.get("const")) {
        (Some(Value::Array(listed)), _) => Some(listed.clone()),
        (_, Some(one)) => Some(vec![one.clone()]),
        _ => None,
    }
}

/// How a variant is named: by a property that has one value and that the variant requires (`kind`
/// first, then `shown`, `verdict` and the like, by name), else the schema it refers to, else its
/// type. A name from a value does not shift when a variant is inserted.
fn label_of(reference: Option<&str>, body: &Value, index: usize) -> String {
    let single = |name: &str| {
        body.pointer(&format!("/properties/{name}"))
            .and_then(values)
            .filter(|listed| listed.len() == 1)
            .map(|listed| match &listed[0] {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            })
    };
    if let Some(value) = single("kind") {
        return format!("kind={value}");
    }
    for name in properties(body).keys() {
        if required(body).contains(name.as_str())
            && let Some(value) = single(name)
        {
            return format!("{name}={value}");
        }
    }
    if let Some(reference) = reference {
        return format!("ref {reference}");
    }
    let types = kinds(body);
    if types.is_empty() {
        format!("#{index}")
    } else {
        types.into_iter().collect::<Vec<_>>().join("|")
    }
}

/// The alternatives of a node, each with its name (made unique) and the node as written.
fn branches(spec: &Value, shape: &Shape) -> Vec<(String, Value)> {
    let listed: Vec<(String, Value)> = if shape.members.is_empty() {
        vec![(
            label_of(shape.reference.as_deref(), &shape.body, 0),
            shape.body.clone(),
        )]
    } else {
        shape
            .members
            .iter()
            .enumerate()
            .map(|(index, member)| {
                let inner = self::shape(spec, member);
                (
                    label_of(inner.reference.as_deref(), &inner.body, index),
                    member.clone(),
                )
            })
            .collect()
    };
    let mut taken = BTreeSet::new();
    listed
        .into_iter()
        .enumerate()
        .map(|(index, (label, node))| {
            let label = if taken.insert(label.clone()) {
                label
            } else {
                format!("{label}#{index}")
            };
            (label, node)
        })
        .collect()
}

/// A member of a node, or the empty schema when it has none.
fn part(node: &Value, key: &str) -> Value {
    node.get(key).cloned().unwrap_or_else(|| json!({}))
}

/// The names of what an object requires.
fn required(body: &Value) -> BTreeSet<&str> {
    body.get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

/// The properties of an object.
fn properties(body: &Value) -> Map<String, Value> {
    body.get("properties")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

/// Whether `additionalProperties` leaves a map open: not there, `true`, or an empty schema.
fn is_open(extra: Option<&Value>) -> bool {
    match extra {
        None | Some(Value::Bool(true)) => true,
        Some(Value::Object(members)) => members.is_empty(),
        Some(_) => false,
    }
}

/// The walk over two descriptions.
pub(super) struct Walk<'a> {
    old: &'a Value,
    new: &'a Value,
    found: Vec<String>,
    /// The pairs of schemas already compared, for each side.
    seen: BTreeSet<(String, String, Side)>,
    /// The frozen schemas that an operation reaches.
    reached: BTreeSet<String>,
}

/// What the frozen description has and the new one lacks or tightened, one line for each.
pub(super) fn broken(frozen: &Value, current: &Value) -> Vec<String> {
    let mut walk = Walk {
        old: frozen,
        new: current,
        found: Vec::new(),
        seen: BTreeSet::new(),
        reached: BTreeSet::new(),
    };
    walk.operations();
    walk.schemas();
    let mut seen = BTreeSet::new();
    walk.found.retain(|line| seen.insert(line.clone()));
    walk.found
}

impl Walk<'_> {
    fn say(&mut self, path: &str, what: impl std::fmt::Display) {
        self.found.push(format!("{path}: {what}"));
    }

    fn operations(&mut self) {
        for operation in api_spec::operations(self.old).into_iter().flatten() {
            let place = operation.place();
            let Some(item) = self.new.get("paths").and_then(|p| p.get(operation.path)) else {
                self.say(&place, "the path is gone");
                continue;
            };
            let Some(now) = item.get(operation.method.to_lowercase()) else {
                self.say(&place, "the operation is gone");
                continue;
            };
            if operation.body.get("operationId") != now.get("operationId") {
                self.say(&place, "the operationId changed");
            }
            let before = self.old.get("paths").and_then(|p| p.get(operation.path));
            self.parameters(&place, before, operation.body, item, now);
            self.request_body(&place, operation.body, now);
            self.answers(&place, operation.body, now);
        }
    }

    /// The parameters of an operation, found by place and name.
    fn parameters(
        &mut self,
        place: &str,
        old_item: Option<&Value>,
        old: &Value,
        new_item: &Value,
        new: &Value,
    ) {
        let listed = |spec: &Value, item: Option<&Value>, operation: &Value| {
            let mut found: Vec<(String, Value)> = Vec::new();
            for source in [item, Some(operation)].into_iter().flatten() {
                for parameter in source
                    .get("parameters")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    let parameter = resolve(spec, parameter);
                    let text = |key: &str| {
                        parameter
                            .get(key)
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned()
                    };
                    found.push((
                        format!("{} {}", text("in"), text("name")),
                        parameter.clone(),
                    ));
                }
            }
            found
        };
        let before = listed(self.old, old_item, old);
        let after = listed(self.new, Some(new_item), new);
        let is_required =
            |parameter: &Value| parameter.get("required").and_then(Value::as_bool) == Some(true);
        for (name, parameter) in &before {
            let path = format!("{place} parameter {name}");
            let Some((_, now)) = after.iter().find(|(other, _)| other == name) else {
                self.say(&path, "the parameter is gone");
                continue;
            };
            if is_required(now) && !is_required(parameter) {
                self.say(&path, "the parameter became required");
            }
            self.compare(
                &path,
                &part(parameter, "schema"),
                &part(now, "schema"),
                Side::Request,
            );
        }
        for (name, parameter) in &after {
            if is_required(parameter) && !before.iter().any(|(other, _)| other == name) {
                self.say(
                    &format!("{place} parameter {name}"),
                    "a new parameter is required, a client of the frozen API does not send it",
                );
            }
        }
    }

    fn request_body(&mut self, place: &str, old: &Value, new: &Value) {
        let body = |spec: &Value, operation: &Value| {
            operation
                .get("requestBody")
                .map(|body| resolve(spec, body).clone())
        };
        let (before, after) = (body(self.old, old), body(self.new, new));
        let is_required =
            |body: &Value| body.get("required").and_then(Value::as_bool) == Some(true);
        let path = format!("{place} requestBody");
        match (before, after) {
            (None, None) => {}
            (Some(_), None) => self.say(&path, "the request body is gone"),
            (None, Some(now)) => {
                if is_required(&now) {
                    self.say(&path, "a new request body is required");
                }
            }
            (Some(was), Some(now)) => {
                if is_required(&now) && !is_required(&was) {
                    self.say(&path, "the request body became required");
                }
                self.media(&path, &was, &now, Side::Request);
            }
        }
    }

    /// The media types of a body or an answer, and their schemas.
    fn media(&mut self, path: &str, old: &Value, new: &Value, side: Side) {
        let Some(listed) = old.get("content").and_then(Value::as_object) else {
            return;
        };
        for (media, content) in listed {
            let here = format!("{path} {media}");
            let Some(now) = new.get("content").and_then(|c| c.get(media)) else {
                self.say(&here, "the media type is gone");
                continue;
            };
            self.compare(&here, &part(content, "schema"), &part(now, "schema"), side);
        }
    }

    fn answers(&mut self, place: &str, old: &Value, new: &Value) {
        let Some(listed) = old.get("responses").and_then(Value::as_object) else {
            return;
        };
        for (status, answer) in listed {
            let path = format!("{place} answer {status}");
            let Some(now) = new.get("responses").and_then(|r| r.get(status)) else {
                self.say(&path, "the answer is gone");
                continue;
            };
            let (was, now) = (
                resolve(self.old, answer).clone(),
                resolve(self.new, now).clone(),
            );
            for header in was
                .get("headers")
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(Map::keys)
            {
                if now.get("headers").and_then(|h| h.get(header)).is_none() {
                    self.say(&path, format!("the header {header} is gone"));
                }
            }
            self.media(&path, &was, &now, Side::Answer);
        }
    }

    /// The schemas that no operation reaches: they stay, and so does what they hold.
    fn schemas(&mut self) {
        for name in api_spec::schema_names(self.old) {
            let path = format!("schema {name}");
            if !api_spec::schema_names(self.new).contains(&name) {
                self.say(&path, "the schema is gone");
                continue;
            }
            if !self.reached.contains(name) {
                let node = json!({"$ref": format!("#/components/schemas/{name}")});
                self.compare(&path, &node, &node, Side::Neither);
            }
        }
    }

    /// Compares two nodes of one side.
    fn compare(&mut self, path: &str, old: &Value, new: &Value, side: Side) {
        let (was, now) = (shape(self.old, old), shape(self.new, new));
        // `null` is judged for each use: a nullable `$ref` and a plain one are the same schema.
        if side == Side::Answer && now.nullable && !was.nullable {
            self.say(path, "null is now a value of the answer");
        }
        if side == Side::Request && was.nullable && !now.nullable {
            self.say(path, "null is no longer accepted");
        }
        if was.reference.is_some() || now.reference.is_some() {
            let key = (
                was.reference.clone().unwrap_or_default(),
                now.reference.clone().unwrap_or_default(),
                side,
            );
            if !self.seen.insert(key) {
                return;
            }
            if side != Side::Neither
                && let Some(name) = &was.reference
            {
                self.reached.insert(name.clone());
            }
        }
        if was.members.is_empty() && now.members.is_empty() {
            self.body(path, &was.body, &now.body, side);
            return;
        }
        if !was.members.is_empty() && !now.members.is_empty() {
            // What `allOf` merged into a node with alternatives (members, `required`) is its own.
            self.body(path, &was.body, &now.body, side);
        } else {
            // A plain object turns into a union or the other way round: its own members must
            // still be somewhere, in the node or in one of the alternatives.
            self.kept_in_union(path, &was, &now);
            if side == Side::Request && was.members.is_empty() {
                self.say(
                    path,
                    "the request is now one of several shapes (oneOf or anyOf)",
                );
            }
        }
        let (before, after) = (branches(self.old, &was), branches(self.new, &now));
        for (label, node) in &before {
            let here = format!("{path} > variant({label})");
            match after.iter().find(|(other, _)| other == label) {
                Some((_, replacement)) => self.compare(&here, node, replacement, side),
                None => self.say(&here, "the variant is gone"),
            }
        }
    }

    /// The members of the plain side's body, each of which must be in the other side's body or in
    /// one of its alternatives.
    fn kept_in_union(&mut self, path: &str, was: &Shape, now: &Shape) {
        let holds = |spec: &Value, shape: &Shape| {
            let mut names: BTreeSet<String> = properties(&shape.body).keys().cloned().collect();
            for member in &shape.members {
                names.extend(properties(&self::shape(spec, member).body).keys().cloned());
            }
            names
        };
        let (before, after) = (holds(self.old, was), holds(self.new, now));
        for name in before.difference(&after) {
            self.say(&format!("{path} > {name}"), "the member is gone");
        }
    }

    /// Compares the keywords of two nodes that have no alternatives.
    fn body(&mut self, path: &str, old: &Value, new: &Value, side: Side) {
        if side == Side::Neither {
            // What no operation reaches keeps its values as well as its members.
            self.listed(path, old, new, side);
        } else {
            self.rules(path, old, new, side);
        }
        self.members(path, old, new, side);
        let (items, more) = (old.get("items"), new.get("items"));
        if items.is_some() || more.is_some() {
            let empty = json!({});
            self.compare(
                &format!("{path}[]"),
                items.unwrap_or(&empty),
                more.unwrap_or(&empty),
                side,
            );
        }
        let (extra, now) = (
            old.get("additionalProperties"),
            new.get("additionalProperties"),
        );
        let is_schema = |value: Option<&Value>| value.is_some_and(Value::is_object);
        let where_values = format!("{path}{{}}");
        if let (Some(extra), Some(now)) = (extra, now)
            && is_schema(Some(extra))
            && is_schema(Some(now))
            && !is_open(Some(extra))
            && !is_open(Some(now))
        {
            self.compare(&where_values, extra, now, side);
        }
        match side {
            Side::Request => {
                if now == Some(&Value::Bool(false)) && extra != Some(&Value::Bool(false)) {
                    self.say(path, "the object is closed (additionalProperties: false)");
                }
                if is_open(extra) && !is_open(now) && now != Some(&Value::Bool(false)) {
                    self.say(&where_values, "the values of an open map get a schema");
                }
            }
            Side::Answer => {
                if !is_open(extra) && is_open(now) && is_schema(extra) {
                    self.say(&where_values, "the values of the map are no longer typed");
                }
            }
            Side::Neither => {}
        }
    }

    /// The type, format, values and limits of two nodes.
    fn rules(&mut self, path: &str, old: &Value, new: &Value, side: Side) {
        let (was, now) = (kinds(old), kinds(new));
        let widened =
            side == Side::Answer && !now.is_empty() && !fits(&now, &was) && !was.is_empty();
        let opened = side == Side::Answer && now.is_empty() && !was.is_empty();
        let narrowed =
            side == Side::Request && !now.is_empty() && (was.is_empty() || !fits(&was, &now));
        if widened || opened || narrowed {
            self.say(path, format!("the type changed from {was:?} to {now:?}"));
        }
        let (format_was, format_now) = (old.get("format"), new.get("format"));
        match side {
            Side::Answer if format_was.is_some() && format_was != format_now => {
                self.say(
                    path,
                    format!("the format changed from {format_was:?} to {format_now:?}"),
                );
            }
            Side::Request if format_now.is_some() && format_was != format_now => {
                self.say(
                    path,
                    format!("the format is now {format_now:?}, was {format_was:?}"),
                );
            }
            _ => {}
        }
        self.listed(path, old, new, side);
        if side == Side::Request {
            self.tightened(path, old, new);
        }
    }

    /// The `enum` and `const` values.
    fn listed(&mut self, path: &str, old: &Value, new: &Value, side: Side) {
        match (values(old), values(new)) {
            (Some(was), Some(now)) => {
                for value in was.iter().filter(|value| !now.contains(value)) {
                    self.say(path, format!("the value {value} is gone"));
                }
            }
            (None, Some(_)) if side == Side::Request => {
                self.say(
                    path,
                    "the request now takes a list of values only (enum or const)",
                );
            }
            _ => {}
        }
    }

    /// The rules that only a request can break by adding or tightening them.
    fn tightened(&mut self, path: &str, old: &Value, new: &Value) {
        if let Some(pattern) = new.get("pattern")
            && old.get("pattern") != Some(pattern)
        {
            self.say(path, format!("the pattern {pattern} is new or changed"));
        }
        if old.get("default") != new.get("default") {
            self.say(path, "the default changed");
        }
        if new.get("multipleOf").is_some() && old.get("multipleOf") != new.get("multipleOf") {
            self.say(path, "multipleOf is new or changed");
        }
        if new.get("uniqueItems") == Some(&Value::Bool(true))
            && old.get("uniqueItems") != Some(&Value::Bool(true))
        {
            self.say(path, "uniqueItems is new");
        }
        for key in [
            "maximum",
            "exclusiveMaximum",
            "maxLength",
            "maxItems",
            "maxProperties",
        ] {
            if let Some(now) = new.get(key).and_then(Value::as_f64)
                && old
                    .get(key)
                    .and_then(Value::as_f64)
                    .is_none_or(|was| now < was)
            {
                self.say(path, format!("{key} {now} is new or lower"));
            }
        }
        for key in [
            "minimum",
            "exclusiveMinimum",
            "minLength",
            "minItems",
            "minProperties",
        ] {
            let free = if key.starts_with("min") && key != "minimum" {
                0.0
            } else {
                f64::NEG_INFINITY
            };
            if let Some(now) = new.get(key).and_then(Value::as_f64)
                && now > old.get(key).and_then(Value::as_f64).unwrap_or(free)
            {
                self.say(path, format!("{key} {now} is new or higher"));
            }
        }
    }

    /// The members of an object: each stays, keeps its requiredness rule and is compared in turn.
    fn members(&mut self, path: &str, old: &Value, new: &Value, side: Side) {
        let (before, after) = (properties(old), properties(new));
        let (needs_was, needs_now) = (required(old), required(new));
        for (name, member) in &before {
            let here = format!("{path} > {name}");
            let Some(now) = after.get(name) else {
                self.say(&here, "the member is gone");
                continue;
            };
            let (was, is) = (
                needs_was.contains(name.as_str()),
                needs_now.contains(name.as_str()),
            );
            if side == Side::Answer && was && !is {
                self.say(&here, "the member is no longer always there");
            }
            if side == Side::Request && is && !was {
                self.say(&here, "the member became required");
            }
            self.compare(&here, member, now, side);
        }
        if side == Side::Request {
            for name in after.keys().filter(|name| !before.contains_key(*name)) {
                if needs_now.contains(name.as_str()) {
                    self.say(
                        &format!("{path} > {name}"),
                        "a new member is required, a client of the frozen API does not send it",
                    );
                }
            }
        }
    }
}
