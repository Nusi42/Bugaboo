//! What a value in a search means for a kind of field: the condition that a value `subject`
//! (a column, or a column of linked rows) is the operand, or comes before or after it.
//!
//! A text that people typed is compared as written, by the bytes; a name that a person sees on screen
//! (a status, a type, a choice, a priority) is looked up by `name_key` among the names this person
//! may know, which the use case put in the request (decision 9 of ADR-0005 amendment M5). No
//! comparison uses `LOWER`, `ILIKE` or a collation, so both databases answer alike.

use bb_app::search::ProjectLink;
use bb_core::issue::Priority;
use bb_core::names::name_key;
use bb_core::readable_id::ReadableId;
use bb_searchlang::{Comparison, Operand};
use sea_query::{Cond, Expr, ExprTrait as _, Query, Value};

use super::source::Domain;
use super::time::{Clock, Range};
use super::{CompileError, Compiler, nothing};
use crate::db::StoredId as _;
use crate::schema::IssueRow;

/// The text of a value, for a field that holds text.
fn operand_text(operand: &Operand) -> String {
    match operand {
        Operand::Text { text, .. } => text.clone(),
        Operand::Number { number, .. } => decimal_text(number.value, number.scale),
        Operand::Date { date, .. } => {
            format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
        }
        Operand::Issue { id, .. } => id.text.clone(),
        Operand::Me { .. } => "me".to_owned(),
        Operand::Time { .. } => String::new(),
    }
}

fn decimal_text(value: i64, scale: u8) -> String {
    if scale == 0 {
        return value.to_string();
    }
    let digits = value.unsigned_abs().to_string();
    let scale = usize::from(scale);
    let padded = format!("{digits:0>width$}", width = scale + 1);
    let (whole, parts) = padded.split_at(padded.len() - scale);
    let sign = if value < 0 { "-" } else { "" };
    format!("{sign}{whole}.{parts}")
}

fn values<T: Into<Value>>(found: impl IntoIterator<Item = T>) -> Vec<Value> {
    found.into_iter().map(Into::into).collect()
}

impl Compiler<'_> {
    /// `subject` is one of these values.
    fn among(subject: Expr, found: Vec<Value>) -> Cond {
        if found.is_empty() {
            nothing()
        } else {
            Cond::all().add(subject.is_in(found))
        }
    }

    /// The things of a domain that an operand names, as the values their column holds, in the
    /// order the domain has them (a level for each, for the domains with an order).
    pub(super) fn levels(&self, domain: Domain) -> Result<Vec<Vec<Value>>, CompileError> {
        Ok(match domain {
            Domain::Priority => Priority::ALL
                .iter()
                .map(|each| vec![Value::from(each.code())])
                .collect(),
            Domain::StatusGroup => self
                .request
                .status_groups
                .iter()
                .map(|(_, keys)| values(keys.clone()))
                .collect(),
            Domain::Choice(field) => self
                .request
                .names
                .choices
                .iter()
                .filter(|(each, _)| *each == field)
                .flat_map(|(_, choices)| choices.iter())
                .map(|(_, id)| vec![Value::from(id.stored())])
                .collect(),
            _ => return Err(CompileError::Unsupported("an order among these values")),
        })
    }

    /// The place of the named thing among the levels, `None` when there is no such name.
    fn level_of(&self, domain: Domain, operand: &Operand) -> Option<usize> {
        let wanted = name_key(&operand_text(operand));
        match domain {
            Domain::Priority => Priority::ALL
                .iter()
                .position(|each| name_key(&each.search_word()) == wanted),
            Domain::StatusGroup => self
                .request
                .status_groups
                .iter()
                .position(|(group, _)| name_key(&group.search_word()) == wanted),
            Domain::Choice(field) => self
                .request
                .names
                .choices
                .iter()
                .find(|(each, _)| *each == field)
                .and_then(|(_, choices)| choices.iter().position(|(key, _)| *key == wanted)),
            _ => None,
        }
    }

    /// `subject` is the thing of the name the operand writes: a status, a type, a project, an
    /// issue, a person, a group, a tag or a label, as the request resolved it for this person.
    fn equals_named(&self, domain: Domain, subject: Expr, operand: &Operand) -> Cond {
        let text = operand_text(operand);
        let key = name_key(&text);
        match domain {
            Domain::Status => Self::among(
                subject,
                values(
                    self.request
                        .names
                        .statuses
                        .iter()
                        .filter(|(name, _)| *name == key)
                        .flat_map(|(_, keys)| keys.clone()),
                ),
            ),
            Domain::Type => Self::among(
                subject,
                values(
                    self.request
                        .names
                        .issue_types
                        .iter()
                        .filter(|(name, _)| *name == key)
                        .flat_map(|(_, ids)| ids.iter().map(|id| id.stored())),
                ),
            ),
            Domain::Project => Self::among(
                subject,
                values(self.project_named(&text).map(|project| project.id.stored())),
            ),
            Domain::Issue => match self.issues_named(&text) {
                Some(found) => Cond::all().add(subject.in_subquery(found)),
                None => nothing(),
            },
            Domain::Person(kind) => match operand {
                Operand::Me { .. } => Cond::all().add(subject.eq(self.request.me.stored())),
                _ => Self::among(
                    subject,
                    values(
                        self.request
                            .names
                            .people
                            .iter()
                            .filter(|(of, name, _)| Some(*of) == kind && *name == key)
                            .flat_map(|(_, _, ids)| ids.iter().map(|id| id.stored())),
                    ),
                ),
            },
            Domain::Group => Self::among(
                subject,
                values(
                    self.request
                        .names
                        .groups
                        .iter()
                        .filter(|(name, _)| *name == key)
                        .flat_map(|(_, ids)| ids.iter().map(|id| id.stored())),
                ),
            ),
            Domain::Tag => Self::among(
                subject,
                values(
                    self.request
                        .names
                        .tags
                        .iter()
                        .filter(|(name, _)| *name == key)
                        .flat_map(|(_, ids)| ids.iter().map(|id| id.stored())),
                ),
            ),
            Domain::Label => Self::among(
                subject,
                values(
                    self.request
                        .names
                        .labels
                        .iter()
                        .filter(|(name, _)| *name == key)
                        .flat_map(|(_, ids)| ids.iter().map(|id| id.stored())),
                ),
            ),
            _ => nothing(),
        }
    }

    /// `subject` is the operand.
    pub(super) fn equals(
        &self,
        domain: Domain,
        subject: Expr,
        operand: &Operand,
    ) -> Result<Cond, CompileError> {
        let text = operand_text(operand);
        let key = name_key(&text);
        Ok(match domain {
            Domain::Text => Cond::all().add(subject.eq(text)),
            Domain::Priority | Domain::StatusGroup | Domain::Choice(_) => {
                match self.level_of(domain, operand) {
                    Some(level) => Self::among(
                        subject,
                        self.levels(domain)?
                            .into_iter()
                            .nth(level)
                            .unwrap_or_default(),
                    ),
                    None => nothing(),
                }
            }
            Domain::Status
            | Domain::Type
            | Domain::Project
            | Domain::Issue
            | Domain::Person(_)
            | Domain::Group
            | Domain::Tag
            | Domain::Label => self.equals_named(domain, subject, operand),
            Domain::YesNo => match key.as_str() {
                bb_searchlang::YES => Cond::all().add(subject.eq(1)),
                bb_searchlang::NO => Cond::all().add(subject.eq(0)),
                _ => nothing(),
            },
            Domain::Number(parts) => match parts_of(operand, parts, Rounding::Exact) {
                Some(number) => Cond::all().add(subject.eq(number)),
                None => nothing(),
            },
            Domain::Time(clock) => {
                let range = self.range_of(operand, clock)?;
                Cond::all()
                    .add(subject.clone().gte(range.from))
                    .add(subject.lt(range.to))
            }
        })
    }

    /// `subject` comes before or after the operand: by the order of the levels, of the numbers or
    /// of the time.
    pub(super) fn compares(
        &self,
        domain: Domain,
        subject: Expr,
        operator: Comparison,
        operand: &Operand,
    ) -> Result<Cond, CompileError> {
        match domain {
            Domain::Priority | Domain::StatusGroup | Domain::Choice(_) => {
                let Some(level) = self.level_of(domain, operand) else {
                    return Ok(nothing());
                };
                let levels = self.levels(domain)?;
                let keep = |at: usize| match operator {
                    Comparison::Less => at < level,
                    Comparison::LessOrEqual => at <= level,
                    Comparison::Greater => at > level,
                    Comparison::GreaterOrEqual => at >= level,
                };
                let chosen = levels
                    .into_iter()
                    .enumerate()
                    .filter(|(at, _)| keep(*at))
                    .flat_map(|(_, level)| level)
                    .collect();
                Ok(Self::among(subject, chosen))
            }
            Domain::Number(parts) => {
                let rounding = match operator {
                    Comparison::Less | Comparison::GreaterOrEqual => Rounding::Up,
                    Comparison::Greater | Comparison::LessOrEqual => Rounding::Down,
                };
                let Some(number) = parts_of(operand, parts, rounding) else {
                    return Ok(nothing());
                };
                Ok(Cond::all().add(match operator {
                    Comparison::Less => subject.lt(number),
                    Comparison::LessOrEqual => subject.lte(number),
                    Comparison::Greater => subject.gt(number),
                    Comparison::GreaterOrEqual => subject.gte(number),
                }))
            }
            Domain::Time(clock) => {
                let range = self.range_of(operand, clock)?;
                Ok(Cond::all().add(match operator {
                    Comparison::Less => subject.lt(range.from),
                    Comparison::LessOrEqual => subject.lt(range.to),
                    Comparison::Greater => subject.gte(range.to),
                    Comparison::GreaterOrEqual => subject.gte(range.from),
                }))
            }
            _ => Err(CompileError::Unsupported("an order among these values")),
        }
    }

    /// The stretch of time an operand stands for.
    pub(super) fn range_of(&self, operand: &Operand, clock: Clock) -> Result<Range, CompileError> {
        match operand {
            Operand::Date { date, .. } => self.day(*date, clock),
            Operand::Time { word, .. } => self.word(*word, clock),
            _ => Err(CompileError::Unsupported("a value that is not a time")),
        }
    }

    /// The project of the person with this short name, as it is written in a search (in any case:
    /// the one rule of `ShortName`).
    pub(super) fn project_named(&self, text: &str) -> Option<&ProjectLink> {
        ProjectLink::named_in(&self.request.projects, text)
    }

    /// The issue an Issue ID names, as the request resolved it for this person (the current short
    /// name of a project they see, an issue they read), held to the sight all the same: none for
    /// a text that is no ID or names nothing they may know.
    pub(super) fn issues_named(&self, text: &str) -> Option<sea_query::SelectStatement> {
        let id: ReadableId = text.parse().ok()?;
        let key = id.to_string();
        let (_, issue) = self
            .request
            .names
            .issues
            .iter()
            .find(|(name, _)| *name == key)?;
        let mut found = Query::select();
        found
            .column((IssueRow::Table, IssueRow::Id))
            .from(IssueRow::Table)
            .cond_where(crate::sight::sight_condition(&self.request.sight))
            .and_where(Expr::col((IssueRow::Table, IssueRow::Id)).eq(issue.stored()));
        Some(found)
    }

    /// The issues named by a reference written in the search, the way a value names them.
    pub(super) fn issues_of(&self, reference: &str) -> sea_query::SelectStatement {
        self.issues_named(reference).unwrap_or_else(|| {
            let mut none = Query::select();
            none.column((IssueRow::Table, IssueRow::Id))
                .from(IssueRow::Table)
                .and_where(Expr::val(1).eq(2));
            none
        })
    }
}

/// The operand as a count of parts (millionths for a decimal), rounded as asked when it does not
/// fall on a part; `None` when it has to be exact and does not, or is no number.
fn parts_of(operand: &Operand, parts: i64, rounding: Rounding) -> Option<i64> {
    let Operand::Number { number, .. } = operand else {
        return None;
    };
    let scaled = i128::from(number.value) * i128::from(parts);
    let divisor = 10_i128.pow(u32::from(number.scale));
    let (whole, rest) = (scaled.div_euclid(divisor), scaled.rem_euclid(divisor));
    let rounded = match (rounding, rest) {
        (Rounding::Exact | Rounding::Up, 0) | (Rounding::Down, _) => Some(whole),
        (Rounding::Exact, _) => None,
        (Rounding::Up, _) => Some(whole + 1),
    };
    rounded.map(|found| i64::try_from(found).unwrap_or(if found < 0 { i64::MIN } else { i64::MAX }))
}

/// How a number that falls between two parts is made one.
#[derive(Clone, Copy)]
enum Rounding {
    Exact,
    Up,
    Down,
}
