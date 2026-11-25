//! The totals of a search (QRY-08, PRIV-13, ADR-0050 decisions 5 and 7): the count of the issues it
//! finds and the sums of *Estimated time*, *Time left*, *Time spent* and own fields of numbers,
//! grouped by one field, each over the issues and the values the reader may see. The grouped list,
//! the columns of a board, the boxes of a dashboard and the tool `search_totals` read this one
//! route; the counts of the groups of a search (`search_groups`) are the same totals without sums.
//!
//! Who may read a summed field where is decided here, once, by the field levels of each project
//! and issue type (`Looked::readable_places`), and handed to the store as data: a value counts
//! only there, and each sum says how many issues gave one (`valued`). A field the reader may read
//! nowhere is refused like a field that does not exist. A group by a field of people shows each
//! person as *Who sees people* lets the reader see them (ADR-0022); people shown alike (by the same
//! role, or as *Someone*) are one group, made by the statement so an issue counts once in it, and
//! the groups of people stand in the order of their names, never of their size (PRIV-09). More
//! groups than a page holds come as pages (M5-F44): the store cuts the page of any other field in
//! its statement, by the place of the last group.

mod count;
mod page;
mod shown;

use base64ct::{Base64UrlUnpadded, Encoding as _};
use bb_core::ErrorCode;
use bb_core::id::UserId;
use bb_policy::{Authorized, Masked, Place as Where};
use serde::Serialize;
use sha2::{Digest as _, Sha256};

pub(crate) use count::{Counted, Grouping};
pub use page::GROUPS_PAGE;
pub use shown::{GroupTotalShown, SumShown, TotalShown};

use self::count::sums_of;
use self::page::{Place, page_after};
use super::readable::SearchScope;
use super::request::{GroupKey, GroupPage, SearchStore, Total, TotalsAsked};
use super::run::{Prepared, ShownKey};
use crate::access::Actor;
use crate::ai::Withheld;
use crate::auth::{AppError, AuthStore, Ctx};

/// The word of a sum of an own field of numbers, before the word of the field (`number:budget`).
const NUMBER_SUM: &str = "number:";

/// A sum a total may ask for, by its word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SumWord {
    /// `count`: how many issues; every row has it, so asking for it adds nothing.
    Count,
    /// `estimated`: *Estimated time*.
    Estimated,
    /// `time_left`: *Time left*.
    TimeLeft,
    /// `time_spent`: *Time spent*.
    TimeSpent,
    /// `number:<field>`: an own field of whole numbers or decimals, by its word in a search.
    Number(String),
}

impl SumWord {
    /// The sum a word names; none for a word that names no sum.
    #[must_use]
    pub fn read(word: &str) -> Option<Self> {
        Some(match word {
            "count" => Self::Count,
            "estimated" => Self::Estimated,
            "time_left" => Self::TimeLeft,
            "time_spent" => Self::TimeSpent,
            other => Self::Number(other.strip_prefix(NUMBER_SUM)?.trim().to_owned()),
        })
    }

    /// The word of the sum, as the answer names it.
    #[must_use]
    pub fn word(&self) -> String {
        match self {
            Self::Count => "count".to_owned(),
            Self::Estimated => "estimated".to_owned(),
            Self::TimeLeft => "time_left".to_owned(),
            Self::TimeSpent => "time_spent".to_owned(),
            Self::Number(field) => format!("{NUMBER_SUM}{field}"),
        }
    }
}

/// How the groups of a total are ordered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GroupOrder {
    /// By their keys, as `Group by` orders them: people by their names.
    #[default]
    Key,
    /// The biggest first. Refused for a field of people: no total ranks people (PRIV-09).
    Count,
}

/// Ask for the totals of a search.
pub struct SearchTotalsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The search.
    pub text: String,
    /// Where the search looks.
    pub scope: SearchScope,
    /// The word of the field to group by; none for the total of all the issues only.
    pub by: Option<String>,
    /// The sums wanted, by their words, in the order the answer gives them.
    pub sums: Vec<String>,
    /// How the groups are ordered.
    pub order: GroupOrder,
    /// Where the page after the last one starts: the position inside the cursor of the page before
    /// (`next_cursor`), opened from its seal by the caller (the list of [`totals_list`]).
    pub cursor: Option<String>,
}

/// The name of the list the cursors of a total are sealed for (`crate::seal`): the person and a
/// hash of what is added up, so a cursor opens for nobody else and for no other total. The one
/// name, for the route and for the tool of AI apps.
#[must_use]
pub fn totals_list(person: UserId, query: &SearchTotalsQuery) -> String {
    let scope = match query.scope {
        SearchScope::Everywhere => String::new(),
        SearchScope::Within(project) => project.to_string(),
    };
    let order = match query.order {
        GroupOrder::Key => "key",
        GroupOrder::Count => "count",
    };
    let mut hash = Sha256::new();
    for part in [
        query.text.as_str(),
        &scope,
        query.by.as_deref().unwrap_or_default(),
        order,
    ]
    .into_iter()
    .chain(query.sums.iter().map(String::as_str))
    {
        hash.update(part.as_bytes());
        hash.update([0]);
    }
    format!(
        "totals:{person}:{}",
        Base64UrlUnpadded::encode_string(&hash.finalize())
    )
}

/// The totals of a search.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SearchTotals {
    /// The words of the sums, in the order of the sums of every row.
    pub sums: Vec<String>,
    /// A page of the groups; empty without a field to group by.
    pub groups: Vec<GroupTotalShown>,
    /// All the issues the search finds together: an issue in several groups counts once.
    pub all: TotalShown,
    /// The position after the last group of the page, when there are more, for the caller to seal
    /// into the cursor of the next page.
    pub next_cursor: Option<String>,
}

impl Masked for SearchTotals {}

/// The totals of a search, grouped by one field: a page of the groups and the total of all.
///
/// # Errors
/// As [`super::search_count`]; `request.invalid` for a field to group by or to add up that the
/// person may not use (as one that does not exist), a word that names no sum or is asked twice,
/// the order by size of a field of people, or a position that is not one.
pub async fn search_totals<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    query: SearchTotalsQuery,
) -> Result<Authorized<SearchTotals>, AppError> {
    let actor = ctx.actor(&query.session_token).await?;
    let prepared = ctx.prepare_search(&actor, &query.text, query.scope).await?;
    totals_page(ctx, &actor, prepared, &query, None).await
}

/// The same for the AI (AI-03, M10-Q20): what is kept from AI is not looked at, grouped by or
/// added up.
///
/// # Errors
/// As [`search_totals`].
pub(crate) async fn search_totals_within<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    actor: &Actor,
    query: &SearchTotalsQuery,
    withheld: &Withheld,
) -> Result<Authorized<SearchTotals>, AppError> {
    let prepared = ctx
        .prepare_search_within(actor, &query.text, query.scope, withheld)
        .await?;
    totals_page(ctx, actor, prepared, query, Some(withheld)).await
}

/// A page of the totals of a prepared search.
async fn totals_page<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    actor: &Actor,
    mut prepared: Prepared,
    query: &SearchTotalsQuery,
    withheld: Option<&Withheld>,
) -> Result<Authorized<SearchTotals>, AppError> {
    let refused = || AppError::Refused(ErrorCode::InvalidRequest);
    let after: Option<Place> = query
        .cursor
        .as_deref()
        .map(|cursor| serde_json::from_str(cursor).map_err(|_| refused()))
        .transpose()?;
    let sums = sums_of(&mut prepared, &query.sums, withheld)?;
    let grouping = match query.by.as_deref() {
        Some(by) => Some(ctx.grouping_of(actor, &mut prepared, by, withheld).await?),
        None => None,
    };
    // No total ranks people: refused before anything is read.
    if query.order == GroupOrder::Count && grouping.as_ref().is_some_and(Grouping::of_people) {
        return Err(refused());
    }
    let shown_of = |total: &Total| TotalShown::of(total, &sums.units);
    let Some(grouping) = grouping else {
        let asked = TotalsAsked {
            by: None,
            sums: sums.asked.clone(),
            all: true,
        };
        let totals = ctx.totals_in(&prepared, &asked).await?;
        let all = shown_of(&totals.all);
        let organization = ctx.evaluation_of(actor, Where::Organization).await?;
        return Ok(answer(&organization, (sums.words, Vec::new(), all), None));
    };
    let counted = if grouping.of_people() {
        ctx.people_totals(&prepared, &grouping, (&sums.asked, true))
            .await?
    } else {
        // In the order of the keys the store reads only the page; by size it reads every group.
        let page = (query.order == GroupOrder::Key).then(|| GroupPage {
            after: after.as_ref().map(|place| place.key().clone()),
            limit: u64::try_from(GROUPS_PAGE + 1).unwrap_or(u64::MAX),
        });
        let totals = ctx
            .key_totals(&prepared, &grouping, (&sums.asked, true), page)
            .await?;
        Counted {
            groups: totals
                .groups
                .into_iter()
                .map(|group| (ShownKey::Other(group.key), group.total))
                .collect(),
            all: totals.all,
        }
    };
    let (page, next_cursor) = page_after(counted.groups, query.order, after.as_ref())?;
    // The keys of other fields are shown for the groups of the page only; people were shown
    // before, to be put in order.
    let raw: Vec<GroupKey> = page
        .iter()
        .filter_map(|(_, key, _)| match key {
            ShownKey::Other(key) if !grouping.of_people() => Some(key.clone()),
            _ => None,
        })
        .collect();
    let mut raw_shown = ctx
        .shown_keys(&grouping.viewer, &prepared.looked, grouping.target, &raw)
        .await?
        .into_iter();
    let mut sums_of_page = Vec::with_capacity(page.len());
    let mut keyed = Vec::with_capacity(page.len());
    for (_, key, total) in page {
        let key = match key {
            ShownKey::Other(_) if !grouping.of_people() => raw_shown.next().unwrap_or(key),
            shown => shown,
        };
        sums_of_page.push(shown_of(&total).sums);
        keyed.push((key, total.issues));
    }
    let shown = ctx
        .groups_with_filters(
            actor,
            &grouping.viewer,
            &prepared.looked,
            (&grouping.word, Some(grouping.target)),
            keyed,
        )
        .await?;
    let groups = shown
        .into_iter()
        .zip(sums_of_page)
        .map(|(group, sums)| GroupTotalShown { group, sums })
        .collect();
    let all = shown_of(&counted.all);
    let organization = ctx.evaluation_of(actor, Where::Organization).await?;
    Ok(answer(
        &organization,
        (sums.words, groups, all),
        next_cursor,
    ))
}

/// The answer of the totals, with the rights of the person for the whole organization.
fn answer(
    organization: &bb_policy::Evaluation,
    (sums, groups, all): (Vec<String>, Vec<GroupTotalShown>, TotalShown),
    next_cursor: Option<String>,
) -> Authorized<SearchTotals> {
    organization.authorize(SearchTotals {
        sums,
        groups,
        all,
        next_cursor,
    })
}
