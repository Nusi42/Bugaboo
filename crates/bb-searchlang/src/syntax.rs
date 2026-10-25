//! The tree of a search. Simple search edits it, Advanced search is the text it prints to, and the
//! checker reads it. It crosses between the browser module and the screen as JSON, never the wire
//! (ADR-0005, amendment M5, decision 4): the canonical text is what travels.
//!
//! Rules that hold in every tree the parser makes (and a tree that is built by hand should keep):
//! an `And` or an `Or` has at least two items and none of its items is of the same kind (the
//! parser takes `(a and b) and c` as `a and b and c`; `Expr::all` and `Expr::any` do the same).
//! A text that is a word of the language only stays a value when it is quoted, which the printer
//! does.

use serde::Serialize;

use crate::span::Span;

/// A whole search: the conditions and the order of the results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Search {
    /// What the results must fulfil; `null` when the search asks for everything.
    pub filter: Option<Expr>,
    /// The order, first key first.
    pub sort: Vec<SortKey>,
    /// The whole text.
    pub span: Span,
}

/// A condition or a combination of conditions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Expr {
    /// At least one of the items holds (`or`).
    Or {
        /// The items, at least two.
        items: Vec<Expr>,
        /// From the first item to the last.
        span: Span,
    },
    /// All of the items hold (`and`).
    And {
        /// The items, at least two.
        items: Vec<Expr>,
        /// From the first item to the last.
        span: Span,
    },
    /// The item does not hold (`not`).
    Not {
        /// The item.
        item: Box<Expr>,
        /// From `not` to the end of the item.
        span: Span,
    },
    /// A field has a value, or a test holds for it.
    Field {
        /// The field.
        field: FieldRef,
        /// What is asked of it.
        test: Test,
        /// From the field to the end of the test.
        span: Span,
    },
    /// The issues below an issue (`under WEB-7`).
    Under {
        /// The issue.
        id: ReadableId,
        /// The whole condition.
        span: Span,
    },
    /// The issues that an issue blocks (`blocks WEB-1`).
    Blocks {
        /// The issue.
        id: ReadableId,
        /// The whole condition.
        span: Span,
    },
    /// The issues that block an issue (`blocked by WEB-1`).
    BlockedBy {
        /// The issue.
        id: ReadableId,
        /// The whole condition.
        span: Span,
    },
    /// Every issue on a chain of relations from an issue (`connected to WEB-7 within 3 steps`).
    ConnectedTo {
        /// The issue.
        id: ReadableId,
        /// How far the chain may go; `null` for as far as the limit allows.
        steps: Option<Steps>,
        /// The whole condition.
        span: Span,
    },
}

/// What is asked of a field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Test {
    /// `is value`.
    Is {
        /// The value.
        value: Operand,
        /// The whole test.
        span: Span,
    },
    /// `is not value`.
    IsNot {
        /// The value.
        value: Operand,
        /// The whole test.
        span: Span,
    },
    /// `< value`, `> value`, `<= value`, `>= value`.
    Compare {
        /// Which comparison.
        operator: Comparison,
        /// The value.
        value: Operand,
        /// The whole test.
        span: Span,
    },
    /// `in (a, b)`.
    In {
        /// The values, at least one.
        values: Vec<Operand>,
        /// The whole test.
        span: Span,
    },
    /// `contains "text"`.
    Contains {
        /// The text to find.
        text: Text,
        /// The whole test.
        span: Span,
    },
    /// `is empty`.
    IsEmpty {
        /// The whole test.
        span: Span,
    },
    /// `is not empty`.
    IsNotEmpty {
        /// The whole test.
        span: Span,
    },
    /// `in last 7 days`.
    InLast {
        /// How many units, at least one.
        count: u32,
        /// The unit.
        unit: Unit,
        /// The whole test.
        span: Span,
    },
    /// `member of "Backend"`.
    MemberOf {
        /// The name of the group.
        group: Text,
        /// The whole test.
        span: Span,
    },
    /// `within WEB`: the project and everything below it.
    Within {
        /// The short name of the project.
        project: ShortName,
        /// The whole test.
        span: Span,
    },
}

/// A comparison of a value with the one in the issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparison {
    /// `<`.
    Less,
    /// `>`.
    Greater,
    /// `<=`.
    LessOrEqual,
    /// `>=`.
    GreaterOrEqual,
}

/// A value in a condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Operand {
    /// Words: a name, a choice, a text. Quoted when it is not one plain word.
    Text {
        /// The text, without quotes.
        text: String,
        /// The word or the quoted text.
        span: Span,
    },
    /// A number.
    Number {
        /// The number.
        number: Decimal,
        /// Where it stands.
        span: Span,
    },
    /// A day.
    Date {
        /// The day.
        date: Date,
        /// Where it stands.
        span: Span,
    },
    /// An issue (`WEB-142`).
    Issue {
        /// The issue.
        id: ReadableId,
        /// Where it stands.
        span: Span,
    },
    /// The person who runs the search.
    Me {
        /// Where it stands.
        span: Span,
    },
    /// A time word. It stays relative in the tree: the server turns it into moments in the zone of
    /// the person's region.
    Time {
        /// The time word.
        word: TimeWord,
        /// Where it stands.
        span: Span,
    },
}

/// A text in a condition that always asks for words (`contains`, `member of`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Text {
    /// The text, without quotes.
    pub text: String,
    /// The word or the quoted text.
    pub span: Span,
}

/// A field, by its word.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldRef {
    /// The word, without quotes.
    pub word: String,
    /// Where it stands.
    pub span: Span,
}

/// An Issue ID: the short name of a project and a code (`WEB-142`, `WEB-K7Q2X`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadableId {
    /// The ID.
    pub text: String,
    /// Where it stands.
    pub span: Span,
}

/// The short name of a project (`WEB`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShortName {
    /// The short name.
    pub text: String,
    /// Where it stands.
    pub span: Span,
}

/// How far a chain of relations may go (`within 3 steps`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Steps {
    /// The number of steps, at least one.
    pub count: u32,
    /// From the count to `steps`.
    pub span: Span,
}

/// A number with a decimal point: `value` divided by ten to the power of `scale`. The shortest
/// form is the only one: no zero ends the fraction, so `1.50` is 15 with scale 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Decimal {
    /// The number without the point.
    pub value: i64,
    /// How many digits stand after the point.
    pub scale: u8,
}

/// A day in the calendar, written `2026-10-25`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Date {
    /// The year, 1 to 9999.
    pub year: u16,
    /// The month, 1 to 12.
    pub month: u8,
    /// The day of the month.
    pub day: u8,
}

/// A time word, relative to the moment of the search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeWord {
    /// `today`.
    Today,
    /// `yesterday`.
    Yesterday,
    /// `this week`.
    ThisWeek,
    /// `last week`.
    LastWeek,
    /// `this month`.
    ThisMonth,
}

/// The unit of `in last 7 days`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    /// Days.
    Day,
    /// Weeks.
    Week,
    /// Months.
    Month,
}

/// One key of `sort by`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SortKey {
    /// The field.
    pub field: FieldRef,
    /// The direction; `null` for the usual one of the field.
    pub direction: Option<Direction>,
    /// From the field to the end of the direction.
    pub span: Span,
}

/// The direction of a sort key, in plain words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// `highest first`.
    HighestFirst,
    /// `lowest first`.
    LowestFirst,
    /// `newest first`.
    NewestFirst,
    /// `oldest first`.
    OldestFirst,
    /// `A to Z`.
    AToZ,
    /// `Z to A`.
    ZToA,
}

impl Expr {
    /// Where the expression stands in the text.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Or { span, .. }
            | Self::And { span, .. }
            | Self::Not { span, .. }
            | Self::Field { span, .. }
            | Self::Under { span, .. }
            | Self::Blocks { span, .. }
            | Self::BlockedBy { span, .. }
            | Self::ConnectedTo { span, .. } => *span,
        }
    }

    /// All of `first` and `rest`: no `And` stands directly in the `And`, and `first` alone is
    /// itself.
    #[must_use]
    pub fn all(first: Self, rest: Vec<Self>) -> Self {
        Self::join(first, rest, true)
    }

    /// At least one of `first` and `rest`: no `Or` stands directly in the `Or`, and `first` alone
    /// is itself.
    #[must_use]
    pub fn any(first: Self, rest: Vec<Self>) -> Self {
        Self::join(first, rest, false)
    }

    fn join(first: Self, rest: Vec<Self>, all: bool) -> Self {
        let mut flat = Vec::new();
        for item in std::iter::once(first).chain(rest) {
            match item {
                Self::And { items, .. } if all => flat.extend(items),
                Self::Or { items, .. } if !all => flat.extend(items),
                other => flat.push(other),
            }
        }
        let span = match (flat.first(), flat.last()) {
            (Some(first), Some(last)) => first.span().to(last.span()),
            _ => Span::default(),
        };
        match flat.pop() {
            Some(only) if flat.is_empty() => only,
            Some(last) => {
                flat.push(last);
                if all {
                    Self::And { items: flat, span }
                } else {
                    Self::Or { items: flat, span }
                }
            }
            None => Self::And { items: flat, span },
        }
    }

    /// Calls `visit` for each condition (not for `and`, `or` and `not`), in the order of the text.
    pub fn each_condition<'a>(&'a self, visit: &mut impl FnMut(&'a Self)) {
        match self {
            Self::Or { items, .. } | Self::And { items, .. } => {
                for item in items {
                    item.each_condition(visit);
                }
            }
            Self::Not { item, .. } => item.each_condition(visit),
            condition => visit(condition),
        }
    }
}

impl Comparison {
    /// Every comparison.
    pub const ALL: [Self; 4] = [
        Self::Less,
        Self::Greater,
        Self::LessOrEqual,
        Self::GreaterOrEqual,
    ];
}

impl Unit {
    /// Every unit.
    pub const ALL: [Self; 3] = [Self::Day, Self::Week, Self::Month];
}
