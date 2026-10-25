//! Reading the tree and the field catalogue back from JSON, by hand.
//!
//! The browser gives the tree (which Simple search edited) and the catalogue to the WebAssembly
//! module as JSON. Deriving `Deserialize` for all these types makes the module a third bigger than
//! its budget allows (ADR-0005, amendment M5, decision 7), so they are read from a `serde_json::Value`
//! here. The shape is the one `Serialize` writes; the tests write every generated tree and read
//! it back, so the two cannot drift. Unit enums are read through their `Serialize` name, so their
//! words stand in one place.

use serde::Serialize;
use serde_json::{Map, Value as Json};

use crate::catalogue::{Field, FieldCatalogue, FieldKind, Operator};
use crate::span::Span;
use crate::syntax::{
    Comparison, Date, Decimal, Direction, Expr, FieldRef, Operand, ReadableId, Search, ShortName,
    SortKey, Steps, Test, Text, TimeWord, Unit,
};

/// What went wrong: the part of the JSON that is not what the tree holds.
pub type Problem = String;

type Read<T> = Result<T, Problem>;

/// Reads a tree.
///
/// # Errors
/// A member is missing or holds something else than the tree does.
pub fn search_from_json(json: &Json) -> Read<Search> {
    let search = object(json, "the search")?;
    Ok(Search {
        filter: optional(member(search, "filter")?, expr)?,
        sort: list(member(search, "sort")?, sort_key)?,
        span: span(search)?,
    })
}

/// Reads a field catalogue.
///
/// # Errors
/// A member is missing or holds something else than the catalogue does.
pub fn catalogue_from_json(json: &Json) -> Read<FieldCatalogue> {
    let catalogue = object(json, "the catalogue")?;
    Ok(FieldCatalogue {
        fields: list(member(catalogue, "fields")?, field)?,
    })
}

fn object<'a>(json: &'a Json, what: &str) -> Read<&'a Map<String, Json>> {
    json.as_object()
        .ok_or_else(|| format!("{what} is not an object"))
}

fn member<'a>(of: &'a Map<String, Json>, key: &str) -> Read<&'a Json> {
    of.get(key).ok_or_else(|| format!("`{key}` is missing"))
}

fn text(of: &Map<String, Json>, key: &str) -> Read<String> {
    member(of, key)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("`{key}` is not a text"))
}

fn whole(of: &Map<String, Json>, key: &str) -> Read<u64> {
    member(of, key)?
        .as_u64()
        .ok_or_else(|| format!("`{key}` is not a whole number"))
}

fn small<T: TryFrom<u64>>(of: &Map<String, Json>, key: &str) -> Read<T> {
    T::try_from(whole(of, key)?).map_err(|_| format!("`{key}` is out of range"))
}

fn list<T>(json: &Json, each: impl Fn(&Json) -> Read<T>) -> Read<Vec<T>> {
    json.as_array()
        .ok_or("a list is expected")?
        .iter()
        .map(each)
        .collect()
}

fn optional<T>(json: &Json, some: impl Fn(&Json) -> Read<T>) -> Read<Option<T>> {
    if json.is_null() {
        Ok(None)
    } else {
        some(json).map(Some)
    }
}

/// A word of a unit enum, found by what `Serialize` writes for each of `all`.
fn word<T: Serialize + Copy>(all: &[T], json: &Json) -> Read<T> {
    all.iter()
        .copied()
        .find(|candidate| serde_json::to_value(candidate).is_ok_and(|written| &written == json))
        .ok_or_else(|| format!("`{json}` is not one of the words that are known here"))
}

fn span(of: &Map<String, Json>) -> Read<Span> {
    let place = object(member(of, "span")?, "`span`")?;
    Ok(Span::new(small(place, "start")?, small(place, "end")?))
}

/// The members of an object with a `type`, and the type.
fn typed<'a>(json: &'a Json, what: &str) -> Read<(&'a str, &'a Map<String, Json>)> {
    let members = object(json, what)?;
    let kind = member(members, "type")?
        .as_str()
        .ok_or("`type` is not a text")?;
    Ok((kind, members))
}

fn unknown<T>(what: &str, kind: &str) -> Read<T> {
    Err(format!("`{kind}` is not a kind of {what}"))
}

fn sort_key(json: &Json) -> Read<SortKey> {
    let key = object(json, "a sort key")?;
    Ok(SortKey {
        field: field_ref(member(key, "field")?)?,
        direction: optional(member(key, "direction")?, |direction| {
            word(&Direction::ALL, direction)
        })?,
        span: span(key)?,
    })
}

fn field_ref(json: &Json) -> Read<FieldRef> {
    let field = object(json, "a field")?;
    Ok(FieldRef {
        word: text(field, "word")?,
        span: span(field)?,
    })
}

fn readable_id(json: &Json) -> Read<ReadableId> {
    let id = object(json, "an Issue ID")?;
    Ok(ReadableId {
        text: text(id, "text")?,
        span: span(id)?,
    })
}

fn short_name(json: &Json) -> Read<ShortName> {
    let name = object(json, "a short name")?;
    Ok(ShortName {
        text: text(name, "text")?,
        span: span(name)?,
    })
}

fn quoted_text(json: &Json) -> Read<Text> {
    let found = object(json, "a text")?;
    Ok(Text {
        text: text(found, "text")?,
        span: span(found)?,
    })
}

fn steps(json: &Json) -> Read<Steps> {
    let found = object(json, "steps")?;
    Ok(Steps {
        count: small(found, "count")?,
        span: span(found)?,
    })
}

fn expr(json: &Json) -> Read<Expr> {
    let (kind, of) = typed(json, "an expression")?;
    let id = || readable_id(member(of, "id")?);
    let place = span(of)?;
    Ok(match kind {
        "or" | "and" => {
            let items = list(member(of, "items")?, expr)?;
            if kind == "or" {
                Expr::Or { items, span: place }
            } else {
                Expr::And { items, span: place }
            }
        }
        "not" => Expr::Not {
            item: Box::new(expr(member(of, "item")?)?),
            span: place,
        },
        "field" => Expr::Field {
            field: field_ref(member(of, "field")?)?,
            test: test(member(of, "test")?)?,
            span: place,
        },
        "under" => Expr::Under {
            id: id()?,
            span: place,
        },
        "blocks" => Expr::Blocks {
            id: id()?,
            span: place,
        },
        "blocked_by" => Expr::BlockedBy {
            id: id()?,
            span: place,
        },
        "connected_to" => Expr::ConnectedTo {
            id: id()?,
            steps: optional(member(of, "steps")?, steps)?,
            span: place,
        },
        other => return unknown("expression", other),
    })
}

fn test(json: &Json) -> Read<Test> {
    let (kind, of) = typed(json, "a test")?;
    let place = span(of)?;
    let value = || operand(member(of, "value")?);
    Ok(match kind {
        "is" => Test::Is {
            value: value()?,
            span: place,
        },
        "is_not" => Test::IsNot {
            value: value()?,
            span: place,
        },
        "compare" => Test::Compare {
            operator: word(&Comparison::ALL, member(of, "operator")?)?,
            value: value()?,
            span: place,
        },
        "in" => Test::In {
            values: list(member(of, "values")?, operand)?,
            span: place,
        },
        "contains" => Test::Contains {
            text: quoted_text(member(of, "text")?)?,
            span: place,
        },
        "is_empty" => Test::IsEmpty { span: place },
        "is_not_empty" => Test::IsNotEmpty { span: place },
        "in_last" => Test::InLast {
            count: small(of, "count")?,
            unit: word(&Unit::ALL, member(of, "unit")?)?,
            span: place,
        },
        "member_of" => Test::MemberOf {
            group: quoted_text(member(of, "group")?)?,
            span: place,
        },
        "within" => Test::Within {
            project: short_name(member(of, "project")?)?,
            span: place,
        },
        other => return unknown("test", other),
    })
}

fn operand(json: &Json) -> Read<Operand> {
    let (kind, of) = typed(json, "a value")?;
    let place = span(of)?;
    Ok(match kind {
        "text" => Operand::Text {
            text: text(of, "text")?,
            span: place,
        },
        "number" => Operand::Number {
            number: number(member(of, "number")?)?,
            span: place,
        },
        "date" => Operand::Date {
            date: date(member(of, "date")?)?,
            span: place,
        },
        "issue" => Operand::Issue {
            id: readable_id(member(of, "id")?)?,
            span: place,
        },
        "me" => Operand::Me { span: place },
        "time" => Operand::Time {
            word: word(&TimeWord::ALL, member(of, "word")?)?,
            span: place,
        },
        other => return unknown("value", other),
    })
}

fn number(json: &Json) -> Read<Decimal> {
    let found = object(json, "a number")?;
    Ok(Decimal {
        value: member(found, "value")?
            .as_i64()
            .ok_or("`value` is not a whole number")?,
        scale: small(found, "scale")?,
    })
}

fn date(json: &Json) -> Read<Date> {
    let found = object(json, "a day")?;
    Ok(Date {
        year: small(found, "year")?,
        month: small(found, "month")?,
        day: small(found, "day")?,
    })
}

fn field(json: &Json) -> Read<Field> {
    let found = object(json, "a field")?;
    Ok(Field {
        word: text(found, "word")?,
        kind: word(&FieldKind::ALL, member(found, "kind")?)?,
        operators: list(member(found, "operators")?, |operator| {
            word(&Operator::ALL, operator)
        })?,
        choices: list(member(found, "choices")?, |choice| {
            choice
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "a choice is not a text".to_owned())
        })?,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::vectors::{ACCEPTED, sample_catalogue};

    #[test]
    fn the_catalogue_reads_back_as_it_was_written() {
        let catalogue = sample_catalogue();
        let json = serde_json::to_value(&catalogue).unwrap();
        assert_eq!(catalogue_from_json(&json), Ok(catalogue));
    }

    #[test]
    fn the_tree_of_every_accepted_text_reads_back_as_it_was_written() {
        for text in ACCEPTED {
            let tree = crate::parse(text).unwrap();
            let json = serde_json::to_value(&tree).unwrap();
            assert_eq!(search_from_json(&json), Ok(tree), "{text}");
        }
    }

    #[test]
    fn what_is_not_the_tree_says_which_part_is_wrong() {
        let unreadable = |json: Json| search_from_json(&json).unwrap_err();
        assert!(unreadable(json!([])).contains("not an object"));
        assert!(unreadable(json!({})).contains("`filter` is missing"));
        let span = json!({"start": 0, "end": 1});
        let wrong = json!({"filter": {"type": "sideways", "span": span}, "sort": [], "span": span});
        assert!(unreadable(wrong).contains("`sideways` is not a kind of expression"));
        let word = json!({"filter": null, "sort": [{"field": {"word": "a", "span": span},
            "direction": "up", "span": span}], "span": span});
        assert!(unreadable(word).contains("not one of the words"));
        let big = json!({"filter": null, "sort": [], "span": {"start": -1, "end": 1}});
        assert!(unreadable(big).contains("`start`"));
    }
}
