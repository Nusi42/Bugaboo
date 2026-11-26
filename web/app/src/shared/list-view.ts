/**
 * How a list shows its results (UX design section 4.2): the columns, their widths and the field the
 * rows are grouped by. How close together the rows are is no part of it: every screen follows the
 * person's *Spacing* in *My settings › Appearance* (ADR-0034, amendment). The address holds only
 * what a person changed, so a project's default list (or a saved search) can change under an open
 * page. The defaults come from the built-in list and from the presentation of a saved search.
 * Everything that turns an address into a view, or a view into an address or a saved
 * presentation, is here (ODR).
 */

import type { Schema } from "@bugaboo/api-client";

import { listWord, projectColumns, searchColumns } from "./list-columns.ts";
import { everyKind, kindsShownOf, kindsShownParts, type KindsShown } from "./relation-map-view.ts";
import type { SearchState } from "./search-address.ts";

/**
 * What `group` says in the address when the person chose no grouping: the built-in list is grouped,
 * so a missing part cannot mean "none". A dash is no word of a field.
 */
export const noGrouping = "-";

/** The word of the field that the built-in lists group by: the status group (UX section 4.2). */
const builtInGroup = listWord.statusGroup;

/** How the results are shown (*Show as*): a list, a timeline or a map. */
export type ShownAs = Schema<"ViewKind">;

/** The options of a timeline: the date fields, *Plan by effort* and the zoom. */
export type TimelineOptions = Schema<"TimelineOptionsBody">;

/** The options of a board: the field of its columns, the field of its rows and the card limits. */
export type BoardOptions = Schema<"BoardOptionsBody">;

/** The zoom of a timeline (*Days · Weeks · Months · Quarters*). */
export type ZoomWord = Schema<"Zoom">;

/**
 * Every kind of view, and whether the screen shows it yet: the compiler stops here when the server
 * gets a new one. The calendar is read on the server; its screen comes later, and until then an
 * address that names it opens the list.
 */
const viewKinds: Readonly<Record<ShownAs, boolean>> = {
  list: true,
  timeline: true,
  map: true,
  board: true,
  calendar: false,
};

/** Every zoom, the same way, in the order they are offered. */
const zooms: Readonly<Record<ZoomWord, true>> = {
  day: true,
  week: true,
  month: true,
  quarter: true,
};

/**
 * Whether a view draws its own Legend below its own tools (the timeline, the relation map and the
 * board), so the row above the results leaves it out: one Legend on the page.
 */
export function drawsOwnLegend(kind: string | undefined): boolean {
  const own: readonly (string | undefined)[] = [
    "timeline",
    "map",
    "board",
  ] satisfies readonly ShownAs[];
  return own.includes(kind);
}

function isViewKind(word: string | undefined): word is ShownAs {
  for (const [kind, shown] of Object.entries(viewKinds)) {
    if (shown && kind === word) {
      return true;
    }
  }
  return false;
}

function isZoom(word: string | undefined): word is ZoomWord {
  return word !== undefined && Object.hasOwn(zooms, word);
}

/** The zooms in the order they are offered. */
export const zoomWords: readonly ZoomWord[] = Object.keys(zooms).filter((word) => isZoom(word));

/** How a list shows its results. */
export interface ListView {
  /** The search words of the columns, in order. */
  readonly columns: readonly string[];
  /** The width of the columns a person dragged, by search word. */
  readonly widths: Readonly<Record<string, number>>;
  /** The search word of the field the rows are grouped by; none for no grouping. */
  readonly groupBy: string | null;
  /** *Show as*: the list, or the timeline. */
  readonly kind: ShownAs;
  /** The options of the timeline, kept while the list is shown. */
  readonly timeline: TimelineOptions;
  /** *Kinds shown* of the relation map of the results (UX §4.15); kept in the address only (M10-F203). */
  readonly map: KindsShown;
  /** The options of the board, kept while another view is shown: only the maker of a saved search changes them. */
  readonly board: BoardOptions;
}

/** What a timeline is grouped by when a list is first shown as one: the goal (UX §4.7, M8-Q12). */
export const timelineStartGroup = "parent";

/** The timeline a saved search has when it says nothing of one: *Start date* and *Due date*, by the week. */
export const builtInTimeline: TimelineOptions = {
  start_field: null,
  end_field: null,
  plan_by_effort: false,
  zoom: "week",
  deadline_drivers: false,
};

/** The board a saved search has when it says nothing of one: a column for each status, no rows, no card limits. */
export const builtInBoard: BoardOptions = { columns: null, rows: null, card_limits: [] };

/** What the built-in list depends on. */
export interface BuiltInInput {
  /** The list is the *Search* page, over every project. */
  readonly everywhere: boolean;
}

/** The list a project or the *Search* page has before anybody changed anything. */
export function builtInView({ everywhere }: BuiltInInput): ListView {
  return {
    columns: everywhere ? searchColumns : projectColumns,
    widths: {},
    groupBy: builtInGroup,
    kind: "list",
    timeline: builtInTimeline,
    map: everyKind,
    board: builtInBoard,
  };
}

/** The timeline an address asks for: what it says, and the defaults for what it leaves out. */
function timelineOf(state: SearchState, defaults: TimelineOptions): TimelineOptions {
  const [start, end] = state.dates ?? [defaults.start_field, defaults.end_field];
  // *Start date* and *Due date* are the view's own when it names no fields (`null`).
  const usual = start === listWord.startDate && end === listWord.dueDate;
  return {
    start_field: usual ? null : start,
    end_field: usual ? null : end,
    plan_by_effort: state.effort ?? defaults.plan_by_effort ?? false,
    zoom: isZoom(state.zoom) ? state.zoom : defaults.zoom,
    deadline_drivers: state.drivers ?? defaults.deadline_drivers ?? false,
  };
}

/** The view an address asks for: what it says, and the defaults for what it leaves out. */
export function viewOf(state: SearchState, defaults: ListView): ListView {
  const { groupBy } = state;
  return {
    columns: state.columns ?? defaults.columns,
    widths: state.widths ?? defaults.widths,
    groupBy: groupBy === undefined ? defaults.groupBy : groupBy === noGrouping ? null : groupBy,
    kind: isViewKind(state.view) ? state.view : defaults.kind,
    timeline: timelineOf(state, defaults.timeline),
    map:
      state.hide === undefined && state.parts === undefined
        ? defaults.map
        : kindsShownOf(state.hide, state.parts),
    board: defaults.board,
  };
}

/**
 * The view with every word of a column, of a width and of the grouping written as the fields the
 * person may read write it (`now`: the old language's `storyPoints` is `story points`), so a list saved before D3 keeps the columns of its own fields.
 */
export function viewInWords(view: ListView, now: (word: string) => string): ListView {
  const widths: Record<string, number> = {};
  for (const word of Object.keys(view.widths)) {
    widths[now(word)] = view.widths[word] ?? 0;
  }
  const { start_field: start, end_field: end } = view.timeline;
  return {
    ...view,
    columns: view.columns.map(now),
    widths,
    groupBy: view.groupBy === null ? null : now(view.groupBy),
    timeline: {
      ...view.timeline,
      start_field: start === null ? null : now(start),
      end_field: end === null ? null : now(end),
    },
    board: {
      ...view.board,
      columns: view.board.columns === null ? null : now(view.board.columns),
      rows: view.board.rows === null ? null : now(view.board.rows),
    },
  };
}

function sameWidths(
  first: Readonly<Record<string, number>>,
  second: Readonly<Record<string, number>>,
): boolean {
  const words = Object.keys(first);
  return (
    words.length === Object.keys(second).length &&
    words.every((word) => first[word] === second[word])
  );
}

/**
 * The state with this view written into it. A part that is what the defaults have is left out, so
 * an address stays short and goes back to the defaults when a person does.
 */
export function withView(state: SearchState, view: ListView, defaults: ListView): SearchState {
  const { text, everything, project, saved, open } = state;
  return {
    text,
    ...(everything === undefined ? {} : { everything }),
    ...(open === undefined ? {} : { open }),
    ...(project === undefined ? {} : { project }),
    ...(saved === undefined ? {} : { saved }),
    ...(view.kind === defaults.kind ? {} : { view: view.kind }),
    ...(view.columns.join(",") === defaults.columns.join(",") ? {} : { columns: view.columns }),
    ...(sameWidths(view.widths, defaults.widths) ? {} : { widths: view.widths }),
    ...(view.groupBy === defaults.groupBy ? {} : { groupBy: view.groupBy ?? noGrouping }),
    ...timelineParts(view.timeline, defaults.timeline),
    ...kindsShownParts(view.map),
  };
}

/** The parts of the address that say how a timeline differs from its defaults. */
function timelineParts(
  timeline: TimelineOptions,
  usual: TimelineOptions,
): Pick<SearchState, "zoom" | "effort" | "drivers" | "dates"> {
  const { start_field: start, end_field: end } = timeline;
  const sameDates = start === usual.start_field && end === usual.end_field;
  const effort = timeline.plan_by_effort ?? false;
  const drivers = timeline.deadline_drivers ?? false;
  // Back to *Start date* and *Due date* from a saved view of own date fields is written too.
  const dates = [start ?? listWord.startDate, end ?? listWord.dueDate] as const;
  return {
    ...(timeline.zoom === usual.zoom ? {} : { zoom: timeline.zoom }),
    ...(effort === (usual.plan_by_effort ?? false) ? {} : { effort }),
    ...(drivers === (usual.deadline_drivers ?? false) ? {} : { drivers }),
    ...(sameDates ? {} : { dates }),
  };
}

/** The board options as a saved search keeps them: left out while they are the starting ones. */
function boardOf(board: BoardOptions): Pick<Schema<"PresentationRef">, "board"> {
  const usual =
    board.columns === null && board.rows === null && (board.card_limits ?? []).length === 0;
  return usual ? {} : { board };
}

/** How a saved search keeps a view (QRY-06): the widths belong to their columns. */
export function presentationOf(view: ListView): Schema<"PresentationRef"> {
  return {
    view: view.kind,
    columns: view.columns.map((field) => ({ field, width: view.widths[field] ?? null })),
    group_by: view.groupBy,
    timeline: view.timeline,
    ...boardOf(view.board),
  };
}

/**
 * The view a saved search keeps. A saved search without columns has the columns of the list it is
 * opened in (the project's), so `defaults` give them. A spacing that an older server still sends
 * is not read: the list follows the person's *Spacing*.
 */
export function viewFromPresentation(
  presentation: Schema<"PresentationRef">,
  defaults: ListView,
): ListView {
  const widths: Record<string, number> = {};
  for (const column of presentation.columns) {
    if (column.width !== null) {
      widths[column.field] = column.width;
    }
  }
  return {
    columns:
      presentation.columns.length === 0
        ? defaults.columns
        : presentation.columns.map((column) => column.field),
    widths,
    groupBy: presentation.group_by,
    kind: presentation.view,
    timeline: presentation.timeline ?? defaults.timeline,
    map: defaults.map,
    board: presentation.board ?? defaults.board,
  };
}
