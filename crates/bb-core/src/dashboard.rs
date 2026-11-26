//! Dashboards (CFG-03, ADR-0050): a name, boxes on a grid, the person who made it and the groups
//! and projects it is shared with. The record, the kinds of box, the rules of the grid, the limits
//! and the ready-made boxes are defined here once; the use cases, the store and the API read them
//! from here.
//!
//! A box is a reader of a search, run as the person who looks at it (ADR-0050, decision 3). The
//! grid has 12 columns; a box is at least 3 wide and 2 high. The places are always packed upwards
//! ([`pack`]): a box left out for a viewer leaves no gap, because the places of the others are the
//! packing of the boxes that viewer sees.

use jiff::Timestamp;
use serde::Serialize;

use crate::id::{DashboardBoxId, DashboardId, SavedSearchId, UserId};
use crate::personal::Entity;
use crate::saved_search::ShareTarget;

/// The columns of the grid of a dashboard.
pub const GRID_COLUMNS: u32 = 12;

/// The narrowest a box can be, in columns.
pub const BOX_WIDTH_MIN: u32 = 3;

/// The lowest a box can be, in rows.
pub const BOX_HEIGHT_MIN: u32 = 2;

/// The highest a box can be, in rows: a box is a part of a page, not a page.
pub const BOX_HEIGHT_MAX: u32 = 8;

/// The most boxes one dashboard holds (ADR-0050, decision 3).
pub const BOXES_MAX: usize = 20;

/// The lowest row a box can start at: every box of a full dashboard stacked at its highest.
pub const GRID_ROWS_MAX: u32 = 160;

/// The most dashboards one person has outside the trash, *Home* included (ADR-0050, decision 3):
/// the 51st is refused, and so is taking one out of the trash past it.
pub const DASHBOARDS_MAX: usize = 50;

/// How many issues a box of *Search results* shows: the first by the search's sort (M10-Q13).
pub const BOX_ISSUES: u32 = 10;

/// How many issues of its search a box of *Recent changes* reads the newest changes of: the ones
/// changed last.
pub const RECENT_ISSUES: u32 = 5;

/// How many changes a box of *Recent changes* shows, the newest first.
pub const RECENT_CHANGES: u32 = 10;

/// How many entries a box of *My time this week* shows.
pub const WEEK_ENTRIES: u32 = 50;

/// How many weeks a *Small timeline* shows from the week before today on.
pub const SMALL_TIMELINE_WEEKS: i32 = 5;

/// How many rows a *Small timeline* shows: one page, read-only.
pub const SMALL_TIMELINE_ROWS: u32 = 10;

words! {
    /// The kind of a box (ADR-0050, decision 4; M10-Q13).
    pub enum BoxKind {
        /// The first issues of the search, by its sort.
        SearchResults => "search-results",
        /// The totals of the search by one field, as bars, or one number.
        NumbersAndChart => "numbers-and-chart",
        /// *Estimated time*, *Time left* and *Time spent* of the search, or the viewer's own time
        /// of this week.
        TimeTotals => "time-totals",
        /// The timeline of the search: one page, nothing dragged.
        SmallTimeline => "small-timeline",
        /// The viewer's own timer.
        MyTimer => "my-timer",
        /// The newest changes of the issues the search finds, as the history writes them.
        RecentChanges => "recent-changes",
    }
}

words_in_api!(
    BoxKind,
    "The kind of a box of a dashboard: search results, numbers and chart, time totals, a small timeline, my timer or recent changes."
);

words! {
    /// How a box of *Numbers and chart* shows its totals: bars, or one number. Charts are never
    /// pies (ADR-0050, decision 5).
    pub enum ChartShape {
        /// One bar for each group of the field, in the order of the groups.
        Bars => "bars",
        /// One number: how many issues the search finds.
        Number => "number",
    }
}

words_in_api!(
    ChartShape,
    "How a box of numbers shows its totals: bars for each group, or one number."
);

words! {
    /// Whose time a box of *Time totals* adds up.
    pub enum TimeSpan {
        /// *Estimated time*, *Time left* and *Time spent* of the issues the search finds.
        Search => "search",
        /// The viewer's own time entries of this week, and nobody else's.
        MyWeek => "my-week",
    }
}

words_in_api!(
    TimeSpan,
    "Whose time a box of time totals adds up: the issues of its search, or the viewer's own week."
);

/// The kinds of box that read no search (ADR-0050, decision 4): the viewer's own timer.
pub const KINDS_WITHOUT_SEARCH: [BoxKind; 1] = [BoxKind::MyTimer];

/// Whose time a box of *Time totals* adds up without a search: the viewer's own week.
pub const SPANS_WITHOUT_SEARCH: [TimeSpan; 1] = [TimeSpan::MyWeek];

/// The kinds of box that show their totals in a shape (bars by a field, or one number) and take
/// a field only as bars.
pub const KINDS_WITH_SHAPE: [BoxKind; 1] = [BoxKind::NumbersAndChart];

/// The kinds of box that say whose time they add up.
pub const KINDS_WITH_SPAN: [BoxKind; 1] = [BoxKind::TimeTotals];

/// Where a box stands on the grid and how big it is, in columns and rows counted from 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct BoxPlace {
    /// The first column, from 0.
    pub column: u32,
    /// The first row, from 0.
    pub row: u32,
    /// How many columns wide.
    pub width: u32,
    /// How many rows high.
    pub height: u32,
}

impl BoxPlace {
    /// Whether the place fits the grid: at least [`BOX_WIDTH_MIN`] × [`BOX_HEIGHT_MIN`], at most
    /// [`GRID_COLUMNS`] wide and [`BOX_HEIGHT_MAX`] high, inside the columns, above
    /// [`GRID_ROWS_MAX`].
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.width >= BOX_WIDTH_MIN
            && self.height >= BOX_HEIGHT_MIN
            && self.height <= BOX_HEIGHT_MAX
            && self.width <= GRID_COLUMNS
            && self.column <= GRID_COLUMNS - self.width
            && self.row < GRID_ROWS_MAX
    }

    /// Whether two places share a cell.
    #[must_use]
    pub const fn overlaps(self, other: Self) -> bool {
        self.column < other.column + other.width
            && other.column < self.column + self.width
            && self.row < other.row + other.height
            && other.row < self.row + self.height
    }
}

/// What a box reads: a search text (the canonical text of the search language, ADR-0005) or a
/// saved search.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum BoxSource {
    /// A search text.
    Text(String),
    /// A saved search, by its ID: run as the viewer, when they may open it.
    Saved(SavedSearchId),
}

/// The options of a box, by its kind: the field and the shape of *Numbers and chart*, whose time
/// *Time totals* adds up. A kind takes only its own options.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub struct BoxOptions {
    /// The field the totals are grouped by, by its search word; none for no groups.
    pub by: Option<String>,
    /// How the totals are shown.
    pub shape: Option<ChartShape>,
    /// Whose time is added up.
    pub span: Option<TimeSpan>,
}

/// A box as it is asked for: everything but the dashboard it is on and its ID.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct BoxSpec {
    /// The kind.
    pub kind: BoxKind,
    /// The place on the grid.
    pub place: BoxPlace,
    /// The title.
    pub title: String,
    /// What it reads; none for a kind that reads no search.
    pub source: Option<BoxSource>,
    /// The options of its kind.
    pub options: BoxOptions,
}

impl BoxSpec {
    /// Whether the box reads a search: every kind but those of [`KINDS_WITHOUT_SEARCH`], unless
    /// whose time it adds up is one of [`SPANS_WITHOUT_SEARCH`] (the viewer's own week).
    #[must_use]
    pub fn reads_a_search(&self) -> bool {
        !KINDS_WITHOUT_SEARCH.contains(&self.kind)
            && self
                .options
                .span
                .is_none_or(|span| !SPANS_WITHOUT_SEARCH.contains(&span))
    }

    /// Whether the box can be kept: its place fits the grid, it has a source exactly when it reads
    /// a search, and it holds the options of its kind and no others. *Numbers and chart* has a
    /// shape, and a field only as bars; *Time totals* says whose time. No option sorts by size or
    /// names a person to rank (ADR-0050, decision 5): there is none. Whether the words and the
    /// texts check for a person is the check of the use case.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let options = &self.options;
        let shape_holds = if KINDS_WITH_SHAPE.contains(&self.kind) {
            match options.shape {
                Some(ChartShape::Bars) => true,
                Some(ChartShape::Number) => options.by.is_none(),
                None => false,
            }
        } else {
            options.shape.is_none() && options.by.is_none()
        };
        let span_holds = options.span.is_some() == KINDS_WITH_SPAN.contains(&self.kind);
        let kind_holds = shape_holds && span_holds;
        self.place.is_valid()
            && kind_holds
            && self.source.is_some() == self.reads_a_search()
            && options
                .by
                .as_deref()
                .is_none_or(crate::saved_search::is_word)
    }
}

/// Whether the boxes can be kept together: at most [`BOXES_MAX`], each valid, none overlapping
/// another.
#[must_use]
pub fn boxes_are_valid(boxes: &[BoxSpec]) -> bool {
    boxes.len() <= BOXES_MAX
        && boxes.iter().all(BoxSpec::is_valid)
        && boxes.iter().enumerate().all(|(at, each)| {
            boxes[..at]
                .iter()
                .all(|earlier| !earlier.place.overlaps(each.place))
        })
}

/// The places packed upwards: each box, from the top left down, moves up to the highest row where
/// it overlaps no box placed before it; its column and size stay. The result depends only on the
/// places given, so the places a viewer is shown are those of a dashboard made of the boxes they
/// see, and a box left out leaves no gap (ADR-0050, decision 3). The order of the answer is the
/// order given.
#[must_use]
pub fn pack(places: &[BoxPlace]) -> Vec<BoxPlace> {
    let mut order: Vec<usize> = (0..places.len()).collect();
    // The index is part of the key, so no two keys are equal and an unstable sort gives the one order.
    order.sort_unstable_by_key(|&at| (places[at].row, places[at].column, at));
    let mut packed: Vec<BoxPlace> = places.to_vec();
    // The places settled so far are the entries of `packed` at the indices already passed.
    for (done, &at) in order.iter().enumerate() {
        let place = places[at];
        let mut row = 0;
        while row < place.row
            && order[..done]
                .iter()
                .any(|&other| packed[other].overlaps(BoxPlace { row, ..place }))
        {
            row += 1;
        }
        packed[at] = BoxPlace { row, ..place };
    }
    packed
}

/// A dashboard (CFG-03).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Dashboard {
    /// The ID, in its address `/d/<id>`.
    #[not_personal]
    pub id: DashboardId,
    /// What it is called. Free text that could name a person.
    #[personal(category = "content", basis = "contract")]
    pub name: String,
    /// The person who made it. After *Remove a person* the ID stays as the link to *Former
    /// user …* on a shared dashboard; a private one goes with the person.
    #[personal(category = "identity", basis = "contract")]
    #[person(author)]
    pub maker: UserId,
    /// Whether it is the maker's *Home* (ADR-0050, decision 6): never shared, one for each person.
    #[not_personal]
    pub home: bool,
    /// When it was made.
    #[not_personal]
    pub created_at: Timestamp,
    /// When it was last changed.
    #[not_personal]
    pub updated_at: Timestamp,
    /// When it was deleted into the trash, while it is there.
    #[not_personal]
    pub deleted_at: Option<Timestamp>,
    /// Counts up with every change, for the check that nobody changed it meanwhile (ADR-0019).
    #[not_personal]
    pub version: i64,
}

/// A box of a dashboard, in the order of the dashboard.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct DashboardBox {
    /// The dashboard. A box is the dashboard's maker's as much as the dashboard is.
    #[not_personal]
    #[person_for(part_of, link = "author")]
    pub dashboard: DashboardId,
    /// The ID of the box, which says nothing of the other boxes.
    #[not_personal]
    pub id: DashboardBoxId,
    /// Its place among the boxes of the dashboard, counted from 0.
    #[not_personal]
    pub position: i64,
    /// The kind, the place, the options of its kind and what it reads. The title and the search
    /// text are content a person wrote; the rest is not personal.
    #[personal(category = "content", basis = "contract")]
    pub spec: BoxSpec,
}

/// A dashboard that is shared with a group or a project.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct DashboardShare {
    /// The dashboard.
    #[not_personal]
    pub dashboard: DashboardId,
    /// The group or the project.
    #[not_personal]
    pub target: ShareTarget,
    /// When it was shared.
    #[not_personal]
    pub shared_at: Timestamp,
}

words! {
    /// The ready-made boxes (ADR-0050, decision 4): the boxes *Add box* offers, each a row of
    /// [`READY_MADE_BOXES`]. The word names its title in the message catalog.
    pub enum ReadyMade {
        /// "My open issues".
        MyOpenIssues => "my-open-issues",
        /// "Due soon".
        DueSoon => "due-soon",
        /// "Open bugs by priority".
        OpenBugsByPriority => "open-bugs-by-priority",
        /// "Issues by status group".
        IssuesByStatusGroup => "issues-by-status-group",
        /// "Progress of a release".
        ProgressOfARelease => "progress-of-a-release",
        /// "My time this week".
        MyTimeThisWeek => "my-time-this-week",
        /// "Time on this search".
        TimeOnThisSearch => "time-on-this-search",
        /// "Small timeline of a search".
        SmallTimelineOfASearch => "small-timeline-of-a-search",
        /// "My timer".
        MyTimer => "my-timer",
        /// "Recent changes in this project".
        RecentChangesInThisProject => "recent-changes-in-this-project",
    }
    messages title = "ready-made-box."
}

words_in_api!(ReadyMade, "A ready-made box of a dashboard.");

/// One ready-made box: data, never code of its own (ADR-0050, decision 4).
#[derive(Clone, Copy, Debug)]
pub struct ReadyMadeBox {
    /// Which one; its title is the message of the word.
    pub which: ReadyMade,
    /// The kind of box.
    pub kind: BoxKind,
    /// The search text, in the canonical spelling; none for a box that reads no search.
    pub text: Option<&'static str>,
    /// The field the totals are grouped by.
    pub by: Option<&'static str>,
    /// How the totals are shown.
    pub shape: Option<ChartShape>,
    /// Whose time is added up.
    pub span: Option<TimeSpan>,
    /// How many columns wide it starts.
    pub width: u32,
    /// How many rows high it starts.
    pub height: u32,
}

impl ReadyMadeBox {
    /// The box as a dashboard keeps it, at `column` and `row`, with `title`.
    #[must_use]
    pub fn spec(&self, title: String, column: u32, row: u32) -> BoxSpec {
        BoxSpec {
            kind: self.kind,
            place: BoxPlace {
                column,
                row,
                width: self.width,
                height: self.height,
            },
            title,
            source: self.text.map(|text| BoxSource::Text(text.to_owned())),
            options: BoxOptions {
                by: self.by.map(str::to_owned),
                shape: self.shape,
                span: self.span,
            },
        }
    }
}

/// A ready-made box with no options, a search text and a size of half the grid.
const fn ready(which: ReadyMade, kind: BoxKind, text: Option<&'static str>) -> ReadyMadeBox {
    ReadyMadeBox {
        which,
        kind,
        text,
        by: None,
        shape: None,
        span: None,
        width: GRID_COLUMNS / 2,
        height: LIST_HEIGHT,
    }
}

/// How high a ready-made list starts, in rows: its first issues or changes.
const LIST_HEIGHT: u32 = 4;

/// How high a ready-made chart starts, in rows: a few bars and the switch to its table.
const CHART_HEIGHT: u32 = 3;

/// The text of the issues that are not done.
const NOT_DONE: &str = "status group is not Done";

/// The ready-made boxes, in the order *Add box* offers them (ADR-0050, decision 4; M10-Q13).
pub const READY_MADE_BOXES: &[ReadyMadeBox] = &[
    ready(
        ReadyMade::MyOpenIssues,
        BoxKind::SearchResults,
        Some("assigned to is me and status group is not Done sort by priority, highest first"),
    ),
    ready(
        ReadyMade::DueSoon,
        BoxKind::SearchResults,
        Some("due date is this week and status group is not Done sort by due date, oldest first"),
    ),
    ReadyMadeBox {
        by: Some("priority"),
        shape: Some(ChartShape::Bars),
        height: CHART_HEIGHT,
        ..ready(
            ReadyMade::OpenBugsByPriority,
            BoxKind::NumbersAndChart,
            Some("type is Bug and status group is not Done"),
        )
    },
    ReadyMadeBox {
        by: Some("status group"),
        shape: Some(ChartShape::Bars),
        height: CHART_HEIGHT,
        ..ready(
            ReadyMade::IssuesByStatusGroup,
            BoxKind::NumbersAndChart,
            Some(""),
        )
    },
    ReadyMadeBox {
        by: Some("status group"),
        shape: Some(ChartShape::Bars),
        height: CHART_HEIGHT,
        ..ready(
            ReadyMade::ProgressOfARelease,
            BoxKind::NumbersAndChart,
            Some("planned for release is not empty"),
        )
    },
    ReadyMadeBox {
        span: Some(TimeSpan::MyWeek),
        height: CHART_HEIGHT,
        ..ready(ReadyMade::MyTimeThisWeek, BoxKind::TimeTotals, None)
    },
    ReadyMadeBox {
        span: Some(TimeSpan::Search),
        height: BOX_HEIGHT_MIN,
        ..ready(
            ReadyMade::TimeOnThisSearch,
            BoxKind::TimeTotals,
            Some(NOT_DONE),
        )
    },
    ReadyMadeBox {
        width: GRID_COLUMNS,
        ..ready(
            ReadyMade::SmallTimelineOfASearch,
            BoxKind::SmallTimeline,
            Some(NOT_DONE),
        )
    },
    ReadyMadeBox {
        height: BOX_HEIGHT_MIN,
        ..ready(ReadyMade::MyTimer, BoxKind::MyTimer, None)
    },
    ready(
        ReadyMade::RecentChangesInThisProject,
        BoxKind::RecentChanges,
        Some(""),
    ),
];

/// The ready-made box of a word.
#[must_use]
pub fn ready_made(which: ReadyMade) -> Option<&'static ReadyMadeBox> {
    READY_MADE_BOXES.iter().find(|entry| entry.which == which)
}

/// The boxes *Home* starts with, each on its place (ADR-0050, decision 6): the blocks of *Home*
/// before it was a dashboard, *Timer* and *Due soon* side by side and *My issues* under them, the
/// whole width.
pub const HOME_BOXES: [(ReadyMade, BoxPlace); 3] = [
    (
        ReadyMade::MyTimer,
        BoxPlace {
            column: 0,
            row: 0,
            width: GRID_COLUMNS / 2,
            height: BOX_HEIGHT_MIN,
        },
    ),
    (
        ReadyMade::DueSoon,
        BoxPlace {
            column: GRID_COLUMNS / 2,
            row: 0,
            width: GRID_COLUMNS / 2,
            height: BOX_HEIGHT_MIN,
        },
    ),
    (
        ReadyMade::MyOpenIssues,
        BoxPlace {
            column: 0,
            row: BOX_HEIGHT_MIN,
            width: GRID_COLUMNS,
            height: LIST_HEIGHT,
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    const fn place(column: u32, row: u32, width: u32, height: u32) -> BoxPlace {
        BoxPlace {
            column,
            row,
            width,
            height,
        }
    }

    fn timer_at(place: BoxPlace) -> BoxSpec {
        BoxSpec {
            kind: BoxKind::MyTimer,
            place,
            title: "Timer".to_owned(),
            source: None,
            options: BoxOptions::default(),
        }
    }

    #[test]
    fn cfg_03_a_box_is_at_least_three_by_two_inside_twelve_columns() {
        assert!(place(0, 0, 3, 2).is_valid());
        assert!(place(9, 0, 3, 2).is_valid());
        assert!(place(0, 0, GRID_COLUMNS, BOX_HEIGHT_MAX).is_valid());
        for bad in [
            place(0, 0, 2, 2),
            place(0, 0, 3, 1),
            place(10, 0, 3, 2),
            place(0, 0, 13, 2),
            place(0, 0, 3, BOX_HEIGHT_MAX + 1),
            place(0, GRID_ROWS_MAX, 3, 2),
        ] {
            assert!(!bad.is_valid(), "{bad:?}");
        }
    }

    #[test]
    fn cfg_03_boxes_may_not_overlap_and_a_dashboard_holds_twenty() {
        let side_by_side = [timer_at(place(0, 0, 6, 2)), timer_at(place(6, 0, 6, 2))];
        assert!(boxes_are_valid(&side_by_side));
        let over = [timer_at(place(0, 0, 6, 2)), timer_at(place(5, 1, 6, 2))];
        assert!(!boxes_are_valid(&over));
        let row_of = |at: u32| timer_at(place(0, at * 2, 3, 2));
        let full: Vec<BoxSpec> = (0..20).map(row_of).collect();
        assert!(boxes_are_valid(&full));
        let more: Vec<BoxSpec> = (0..21).map(row_of).collect();
        assert!(!boxes_are_valid(&more));
    }

    #[test]
    fn cfg_03_each_kind_takes_its_own_options_and_none_sorts_by_size() {
        let chart = |by: Option<&str>, shape: Option<ChartShape>| BoxSpec {
            kind: BoxKind::NumbersAndChart,
            place: place(0, 0, 6, 4),
            title: "Open bugs".to_owned(),
            source: Some(BoxSource::Text(String::new())),
            options: BoxOptions {
                by: by.map(str::to_owned),
                shape,
                span: None,
            },
        };
        assert!(chart(Some("assignedTo"), Some(ChartShape::Bars)).is_valid());
        assert!(chart(None, Some(ChartShape::Number)).is_valid());
        assert!(!chart(Some("assignedTo"), Some(ChartShape::Number)).is_valid());
        assert!(!chart(None, None).is_valid());
        let mut timer = timer_at(place(0, 0, 3, 2));
        assert!(timer.is_valid());
        timer.source = Some(BoxSource::Text(String::new()));
        assert!(!timer.is_valid(), "a timer reads no search");
        let mut week = timer_at(place(0, 0, 3, 2));
        week.kind = BoxKind::TimeTotals;
        week.options.span = Some(TimeSpan::MyWeek);
        assert!(week.is_valid());
        week.options.span = Some(TimeSpan::Search);
        assert!(!week.is_valid(), "the time of a search needs a search");
    }

    #[test]
    fn cfg_03_packing_closes_every_gap_and_depends_only_on_the_boxes_kept() {
        let top = place(0, 0, 6, 2);
        let hidden = place(6, 0, 6, 4);
        let under_hidden = place(6, 4, 6, 2);
        let under_top = place(0, 6, 12, 2);
        let all = [top, hidden, under_hidden, under_top];
        assert_eq!(
            pack(&all),
            [top, hidden, under_hidden, place(0, 6, 12, 2)],
            "a packed grid stays as it is"
        );
        let kept = [top, under_hidden, under_top];
        let packed = pack(&kept);
        assert_eq!(
            packed,
            [top, place(6, 0, 6, 2), place(0, 2, 12, 2)],
            "the box under the hidden one moves up, and so does the one below"
        );
        assert_eq!(pack(&packed), packed);
    }

    #[test]
    fn cfg_03_the_ready_made_boxes_are_valid_and_named_once() {
        let mut row = 0;
        for entry in READY_MADE_BOXES {
            let spec = entry.spec("Title".to_owned(), 0, row);
            assert!(spec.is_valid(), "{:?}", entry.which);
            row += entry.height;
        }
        assert_eq!(READY_MADE_BOXES.len(), ReadyMade::ALL.len());
        for which in ReadyMade::ALL {
            assert!(ready_made(*which).is_some(), "{which:?}");
        }
        assert!(
            HOME_BOXES
                .iter()
                .all(|(which, _)| ready_made(*which).is_some())
        );
        let home: Vec<BoxPlace> = HOME_BOXES.iter().map(|(_, place)| *place).collect();
        assert_eq!(pack(&home), home, "the places of Home are packed");
    }
}
