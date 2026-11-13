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
import type { SearchState } from "./search-address.ts";

/**
 * What `group` says in the address when the person chose no grouping: the built-in list is grouped,
 * so a missing part cannot mean "none". A dash is no word of a field.
 */
export const noGrouping = "-";

/** The word of the field that the built-in lists group by: the status group (UX section 4.2). */
const builtInGroup = listWord.statusGroup;

/** How the results are shown (*Show as*): a list or a timeline. */
export type ShownAs = Schema<"ViewKind">;

/** The options of a timeline: the date fields, *Plan by effort* and the zoom. */
export type TimelineOptions = Schema<"TimelineOptionsBody">;

/** The zoom of a timeline (*Days · Weeks · Months · Quarters*). */
export type ZoomWord = Schema<"Zoom">;

/** Every kind of view: the compiler stops here when the server gets a new one. */
const viewKinds: Readonly<Record<ShownAs, true>> = { list: true, timeline: true };

/** Every zoom, the same way, in the order they are offered. */
const zooms: Readonly<Record<ZoomWord, true>> = {
  day: true,
  week: true,
  month: true,
  quarter: true,
};

function isViewKind(word: string | undefined): word is ShownAs {
  return word !== undefined && Object.hasOwn(viewKinds, word);
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
}

/** What a timeline is grouped by when a list is first shown as one: the goal (UX §4.7, M8-Q12). */
export const timelineStartGroup = "parent";

/** The timeline a saved search has when it says nothing of one: *Start date* and *Due date*, by the week. */
export const builtInTimeline: TimelineOptions = {
  start_field: null,
  end_field: null,
  plan_by_effort: false,
  zoom: "week",
};

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
  };
}

/** The parts of the address that say how a timeline differs from its defaults. */
function timelineParts(
  timeline: TimelineOptions,
  usual: TimelineOptions,
): Pick<SearchState, "zoom" | "effort" | "dates"> {
  const { start_field: start, end_field: end } = timeline;
  const sameDates = start === usual.start_field && end === usual.end_field;
  const effort = timeline.plan_by_effort ?? false;
  // Back to *Start date* and *Due date* from a saved view of own date fields is written too.
  const dates = [start ?? listWord.startDate, end ?? listWord.dueDate] as const;
  return {
    ...(timeline.zoom === usual.zoom ? {} : { zoom: timeline.zoom }),
    ...(effort === (usual.plan_by_effort ?? false) ? {} : { effort }),
    ...(sameDates ? {} : { dates }),
  };
}

/** How a saved search keeps a view (QRY-06): the widths belong to their columns. */
export function presentationOf(view: ListView): Schema<"PresentationRef"> {
  return {
    view: view.kind,
    columns: view.columns.map((field) => ({ field, width: view.widths[field] ?? null })),
    group_by: view.groupBy,
    timeline: view.timeline,
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
  };
}
