//! What a box answers (ADR-0050, decisions 3 to 5): each kind calls the read that exists for it,
//! with the session of the viewer, so a box never shows more than the viewer would see by asking
//! that read themselves. No box has a read of its own, and none ranks people: totals stand in the
//! order of their groups (people by their names), and *Recent changes* takes no person.

use bb_core::ErrorCode;
use bb_core::dashboard::{
    BOX_HEIGHT_MIN, BOX_ISSUES, BOX_WIDTH_MIN, BoxKind, BoxOptions, BoxPlace, BoxSource, BoxSpec,
    ChartShape, RECENT_CHANGES, RECENT_ISSUES, SMALL_TIMELINE_ROWS, SMALL_TIMELINE_WEEKS, TimeSpan,
    WEEK_ENTRIES, boxes_are_valid,
};
use bb_core::history::HistoryTab;
use bb_core::id::IssueId;
use bb_core::saved_search::TimelineOptions;
use bb_policy::{Authorized, Place};
use bb_searchlang::{
    Direction, FieldRef, IssueField, SortKey, Span, canonical_text, check, parse_with,
};
use jiff::ToSpan as _;

use super::open::{Opened, Reads};
use super::types::{BoxAnswer, BoxPreviewQuery, BoxQuery, RecentChange, RecentChanges};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::history::{HistoryItem, HistoryQuery, history};
use crate::plan::{TimelinePart, TimelineQuery, timeline};
use crate::saved_searches::reach::Reaching;
use crate::search::{
    GroupOrder, SearchQuery, SearchScope, SearchTotalsQuery, SumWord, ViewSearch, search,
    search_totals,
};
use crate::time::{MyTimeEntriesQuery, TimerQuery, my_time_entries, timer};

/// The value of an `Authorized` whose type hides nothing by its mask (`Masked` with no member of
/// its own): what the read already left out for the viewer is all there is to leave out.
fn value_of<T>(authorized: Authorized<T>) -> Option<T> {
    let mut taken = None;
    let _ = authorized.map(|value| taken = Some(value));
    taken
}

impl<S: AuthStore> Ctx<'_, S> {
    /// `text` sorted by the last change, newest first: what *Recent changes* reads the issues of.
    /// The filter stays as it is.
    async fn newest_first(
        &self,
        actor: &Actor,
        reaching: &Reaching,
        text: &str,
    ) -> Result<String, AppError> {
        let judge = self.judge(actor, reaching).await?;
        let fields = judge.fields();
        let not_read = || AppError::Refused(ErrorCode::InvalidRequest);
        let mut search = parse_with(text, fields).map_err(|_| not_read())?;
        search.sort = vec![SortKey {
            field: FieldRef {
                word: IssueField::Changed.word().to_owned(),
                span: Span::default(),
            },
            direction: Some(Direction::NewestFirst),
            span: Span::default(),
        }];
        let checked = check(search, fields).map_err(|_| not_read())?;
        canonical_text(checked.search()).ok_or_else(not_read)
    }

    /// The newest changes of the issues a search finds (ADR-0050, decision 4): the issues changed
    /// last, each read through its history as the viewer reads it (fields they may not read left
    /// out, people by *Who sees people*), newest first.
    async fn recent_changes(
        &self,
        (token, actor, reaching): (&str, &Actor, &Reaching),
        text: &str,
    ) -> Result<Authorized<RecentChanges>, AppError> {
        let newest = self.newest_first(actor, reaching, text).await?;
        let found = search(
            self,
            SearchQuery {
                session_token: token.to_owned(),
                text: newest,
                scope: SearchScope::Everywhere,
                cursor: None,
                limit: RECENT_ISSUES,
                labels: false,
                releases: false.into(),
                agent: false.into(),
                time: false,
            },
        )
        .await?;
        let mut issues: Vec<(IssueId, String)> = Vec::new();
        for issue in value_of(found).map(|page| page.items).unwrap_or_default() {
            let _ = issue.map(|view| issues.push((view.id, view.key)));
        }
        let mut items: Vec<RecentChange> = Vec::new();
        for (id, key) in issues {
            let page = history(
                self,
                HistoryQuery {
                    session_token: token.to_owned(),
                    issue: id.to_string(),
                    tab: HistoryTab::Changes,
                    cursor: None,
                    limit: RECENT_CHANGES,
                },
            )
            .await?;
            let rows: Vec<HistoryItem> = value_of(page).map(|page| page.items).unwrap_or_default();
            items.extend(rows.into_iter().map(|item| RecentChange {
                issue: key.clone(),
                item,
            }));
        }
        items.sort_by_key(|change| std::cmp::Reverse(change.item.at));
        items.truncate(usize::try_from(RECENT_CHANGES).unwrap_or(usize::MAX));
        Ok(self
            .evaluation_of(actor, Place::Organization)
            .await?
            .authorize(RecentChanges { items }))
    }

    /// What a box answers for the viewer of `token`, through the read of its kind.
    async fn box_answer(
        &self,
        (token, actor, reaching): (&str, &Actor, &Reaching),
        spec: &BoxSpec,
        reads: Reads,
    ) -> Result<BoxAnswer, AppError> {
        let text = match reads {
            Reads::Text(text) => text,
            Reads::Nothing => String::new(),
        };
        let session_token = token.to_owned();
        let totals = |by: Option<String>, sums: Vec<String>| SearchTotalsQuery {
            session_token: session_token.clone(),
            text: text.clone(),
            scope: SearchScope::Everywhere,
            by,
            sums,
            // The order of the groups, never their size: no box ranks people (ADR-0050, decision 5).
            order: GroupOrder::Key,
            cursor: None,
        };
        Ok(match (spec.kind, spec.options.span) {
            (BoxKind::SearchResults, _) => BoxAnswer::Issues(
                search(
                    self,
                    SearchQuery {
                        session_token,
                        text,
                        scope: SearchScope::Everywhere,
                        cursor: None,
                        limit: BOX_ISSUES,
                        labels: false,
                        releases: false.into(),
                        agent: false.into(),
                        time: false,
                    },
                )
                .await?,
            ),
            (BoxKind::NumbersAndChart, _) => {
                let by = match spec.options.shape {
                    Some(ChartShape::Bars) => spec.options.by.clone(),
                    Some(ChartShape::Number) | None => None,
                };
                BoxAnswer::Totals(search_totals(self, totals(by, Vec::new())).await?)
            }
            (BoxKind::TimeTotals, Some(TimeSpan::MyWeek)) => BoxAnswer::Week(
                my_time_entries(
                    self,
                    MyTimeEntriesQuery {
                        session_token,
                        from: None,
                        to: None,
                        cursor: None,
                        limit: WEEK_ENTRIES,
                    },
                )
                .await?,
            ),
            (BoxKind::TimeTotals, _) => {
                let sums = [SumWord::Estimated, SumWord::TimeLeft, SumWord::TimeSpent]
                    .iter()
                    .map(SumWord::word)
                    .collect();
                BoxAnswer::Totals(search_totals(self, totals(None, sums)).await?)
            }
            (BoxKind::SmallTimeline, _) => {
                let today = self.today_of(actor).await?;
                let first = today.saturating_sub(7.days());
                let last = first
                    .saturating_add((SMALL_TIMELINE_WEEKS * 7).days())
                    .saturating_sub(1.day());
                BoxAnswer::Timeline(
                    timeline(
                        self,
                        TimelineQuery {
                            session_token,
                            search: ViewSearch::Text(text),
                            project: None,
                            options: TimelineOptions::default(),
                            group_by: None,
                            first,
                            last,
                            part: TimelinePart::Rows,
                            cursor: None,
                            limit: SMALL_TIMELINE_ROWS,
                        },
                    )
                    .await?,
                )
            }
            (BoxKind::MyTimer, _) => {
                BoxAnswer::Timer(timer(self, TimerQuery { session_token }).await?)
            }
            (BoxKind::RecentChanges, _) => {
                BoxAnswer::Changes(self.recent_changes((token, actor, reaching), &text).await?)
            }
        })
    }

    /// What a box reads for the actor, or `None` when it does not check for them.
    async fn reads_for(
        &self,
        actor: &Actor,
        reaching: &Reaching,
        spec: &BoxSpec,
    ) -> Result<Option<Reads>, AppError> {
        let mut judge = self.judge(actor, reaching).await?;
        judge.lift_budget();
        self.box_reads(&mut judge, reaching, spec).await
    }
}

/// The box judged in place of one that is not on the dashboard: search results of everything.
fn stand_in() -> BoxSpec {
    BoxSpec {
        kind: BoxKind::SearchResults,
        place: BoxPlace {
            column: 0,
            row: 0,
            width: BOX_WIDTH_MIN,
            height: BOX_HEIGHT_MIN,
        },
        title: String::new(),
        source: Some(BoxSource::Text(String::new())),
        options: BoxOptions::default(),
    }
}

/// What one box of a dashboard answers for the person who looks at it: the read of its kind, run as
/// them. A box that does not check for them is not found, as one that does not exist.
///
/// # Errors
/// `resource.not-found` for a dashboard they may not open, or a box that is not on it or does not
/// check for them; the errors of the read of the box.
pub async fn dashboard_box<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: BoxQuery,
) -> Result<BoxAnswer, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let Opened { stored, reaching } = ctx.open_dashboard(&actor, input.dashboard).await?;
    let found = stored
        .boxes
        .into_iter()
        .find(|each| each.id == input.dashboard_box)
        .map(|each| each.spec);
    // A box that is not there is judged like one that is (a stand-in of search results), so a box
    // left out and a box that does not exist run the same reads before the same answer.
    let judged = found.clone().unwrap_or_else(stand_in);
    let reads = ctx.reads_for(&actor, &reaching, &judged).await?;
    let (Some(spec), Some(reads)) = (found, reads) else {
        return Err(ErrorCode::NotFound.into());
    };
    ctx.box_answer((&input.session_token, &actor, &reaching), &spec, reads)
        .await
}

/// What a box would answer before it is added (*Add box*): the preview is made by the same reads
/// as the box itself.
///
/// # Errors
/// `request.invalid` for a box that cannot be kept or does not check for the person; the errors of
/// the read of the box.
pub async fn box_preview<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: BoxPreviewQuery,
) -> Result<BoxAnswer, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    if !boxes_are_valid(std::slice::from_ref(&input.spec)) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let reaching = ctx.reaching(&actor).await?;
    let reads = ctx
        .reads_for(&actor, &reaching, &input.spec)
        .await?
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    ctx.box_answer(
        (&input.session_token, &actor, &reaching),
        &input.spec,
        reads,
    )
    .await
}
