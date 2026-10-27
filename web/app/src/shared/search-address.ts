/**
 * The search in the address (UX design section 2.1, ADR-0005 amendment M5, decision 4): the
 * canonical text travels in `s`, and how the results are shown in `show`, `cols`, `widths`,
 * `spacing` and `group`. {@link readSearch} is the one reader and {@link searchLocation} the one
 * writer, so a list, a board, the *Search* page, a saved search and the search bar can never
 * disagree about the address (ODR). Every route that has a search bar takes its `validateSearch`
 * from here, and every change of the search is written through here, so no part of the address is
 * lost when another changes.
 */

import {
  partOfAddress,
  paths,
  projectIssuesAddress,
  routeParam,
  savedSearchAddress,
} from "./paths.ts";

/**
 * How a change of the search enters the history of the browser (UX design section 2.1): typing in
 * Advanced search replaces the step it began, so Back is not a step per letter; a change in Simple
 * search, and the first text of a stretch of typing, add a step.
 */
export const historyStep = { push: "push", replace: "replace" } as const;
export type HistoryStep = (typeof historyStep)[keyof typeof historyStep];

/** How close together the rows are: the two words of the setting `appearance.spacing`. */
export type SearchSpacing = "comfortable" | "compact";

/** What an address says about a search and how its results are shown. */
export interface SearchState {
  readonly text: string;
  /** The short name of the project the list belongs to; none for the other pages. */
  readonly project?: string;
  /** The ID of the saved search whose page this is (`/s/<ID>`). */
  readonly saved?: string;
  readonly columns?: readonly string[];
  readonly widths?: Readonly<Record<string, number>>;
  readonly spacing?: SearchSpacing;
  readonly groupBy?: string;
  /** How it is shown (`list`, `board` …). */
  readonly view?: string;
}

/** The parts of the address after the question mark that belong to a search, as written. */
export interface SearchParts {
  readonly s?: string;
  readonly show?: string;
  readonly cols?: string;
  readonly widths?: string;
  readonly spacing?: string;
  readonly group?: string;
}

/** Where a search state is written: the path and the parts after the question mark. */
export interface SearchLocation {
  readonly pathname: string;
  readonly search: SearchParts;
}

/** A part of the address as text. The router reads `7` as a number; the text is what was written. */
function textOf(raw: object, name: keyof SearchParts): string | undefined {
  const value: unknown = Reflect.get(raw, name);
  if (typeof value === "string") {
    return value;
  }
  return typeof value === "number" || typeof value === "boolean" ? String(value) : undefined;
}

function widthsOf(written: string | undefined): Record<string, number> | undefined {
  if (written === undefined) {
    return undefined;
  }
  const widths: Record<string, number> = {};
  for (const pair of written.split(",")) {
    const [field = "", width = ""] = pair.split(":");
    if (field !== "" && /^\d+$/u.test(width)) {
      widths[field] = Number(width);
    }
  }
  return widths;
}

/**
 * Reads a search from an address (the one place that does): the path says the project or the
 * saved search, `s` the text, and the other parts how the results are shown. `search` is what the
 * router read. A part that is not there, or is empty, is not in the state.
 */
export function readSearch(location: {
  readonly pathname?: string;
  readonly search: object;
}): SearchState {
  const raw = location.search;
  const pathname = location.pathname ?? "";
  const written = textOf(raw, "s");
  const project = partOfAddress(paths.projectIssues, routeParam.project, pathname);
  const saved = partOfAddress(paths.savedSearch, routeParam.search, pathname);
  const columns = textOf(raw, "cols")
    ?.split(",")
    .filter((each) => each !== "");
  const widths = widthsOf(textOf(raw, "widths"));
  const spacing = textOf(raw, "spacing");
  const groupBy = textOf(raw, "group");
  const view = textOf(raw, "show");
  return {
    text: written === undefined || written.trim() === "" ? "" : written,
    ...(project === undefined ? {} : { project }),
    ...(saved === undefined ? {} : { saved }),
    ...(columns === undefined || columns.length === 0 ? {} : { columns }),
    ...(widths === undefined ? {} : { widths }),
    ...(spacing === "comfortable" || spacing === "compact" ? { spacing } : {}),
    ...(groupBy === undefined || groupBy === "" ? {} : { groupBy }),
    ...(view === undefined ? {} : { view }),
  };
}

/** The address of a search state: the inverse of {@link readSearch}, and the only place that writes it. */
export function searchLocation(state: SearchState): SearchLocation {
  const search: { -readonly [Part in keyof SearchParts]: string } = {};
  if (state.text.trim() !== "") {
    search.s = state.text;
  }
  if (state.view !== undefined) {
    search.show = state.view;
  }
  if (state.columns !== undefined && state.columns.length > 0) {
    search.cols = state.columns.join(",");
  }
  const widths: string[] = [];
  for (const [field, width] of Object.entries(state.widths ?? {})) {
    widths.push(`${field}:${String(width)}`);
  }
  if (widths.length > 0) {
    search.widths = widths.join(",");
  }
  if (state.spacing !== undefined) {
    search.spacing = state.spacing;
  }
  if (state.groupBy !== undefined) {
    search.group = state.groupBy;
  }
  let pathname: string = paths.search;
  if (state.saved !== undefined) {
    pathname = savedSearchAddress(state.saved);
  } else if (state.project !== undefined) {
    pathname = projectIssuesAddress(state.project);
  }
  return { pathname, search };
}

/**
 * What the router keeps of the search part of an address: the parts of a search, normalized, and
 * nothing else. This is the `validateSearch` of every route that has a search bar.
 */
export function validateSearch(raw: object): SearchParts {
  return searchLocation(readSearch({ search: raw })).search;
}

/** The address of a search state as one text, `/search?s=status%20is%20Open`, for a link or a navigation. */
export function searchHref(state: SearchState): string {
  const { pathname, search } = searchLocation(state);
  const query = new URLSearchParams(Object.entries(search)).toString();
  return query === "" ? pathname : `${pathname}?${query}`;
}
