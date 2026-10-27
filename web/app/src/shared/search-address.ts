/**
 * The search in the address (UX design section 2.1, ADR-0005 amendment M5, decision 4): the
 * canonical text travels in `s`, and how the results are shown in `show`. This file is the one
 * place that reads and writes them, so a list, a board and the search bar can never disagree about
 * the address (ODR); every route that has a search bar takes its `validateSearch` from here.
 */

/** The name of the part of the address that holds the canonical text of the search. */
export const searchParam = "s";

/** The name of the part of the address that says how the results are shown (`list`, `board`, …). */
export const viewParam = "show";

/** The search and the view as the address holds them; a part that is not there is `undefined`. */
export interface SearchAddress {
  readonly s?: string | undefined;
  readonly show?: string | undefined;
}

/** A part of the address as text. The router reads `7` as a number; the text is what was written. */
function textOf(value: unknown): string | undefined {
  if (typeof value === "string") {
    return value;
  }
  return typeof value === "number" || typeof value === "boolean" ? String(value) : undefined;
}

/** What the address says about the search; anything else in it is left to the page. */
export function readSearchAddress(raw: Readonly<Record<string, unknown>>): SearchAddress {
  const text = textOf(raw[searchParam]);
  return {
    [searchParam]: text === undefined || text.trim() === "" ? undefined : text,
    [viewParam]: textOf(raw[viewParam]),
  };
}

/** The text of the search that the address holds; the empty text finds everything. */
export function searchTextOf(address: SearchAddress): string {
  return address.s ?? "";
}

/** The address with another search text in it; the empty text takes `s` out. */
export function withSearchText(address: SearchAddress, text: string): SearchAddress {
  return { ...address, [searchParam]: text.trim() === "" ? undefined : text };
}

/** `path` with the search and the view written as the address says, e.g. `/search?s=status%20is%20Open`. */
export function searchHref(path: string, address: SearchAddress): string {
  const query = new URLSearchParams();
  if (address.s !== undefined) {
    query.set(searchParam, address.s);
  }
  if (address.show !== undefined) {
    query.set(viewParam, address.show);
  }
  const written = query.toString();
  return written === "" ? path : `${path}?${written}`;
}

/** How close together the rows are: the two words of the setting `appearance.spacing`. */
export type SearchSpacing = "comfortable" | "compact";

/** The names of the view parts of the address next to `s` (UX design section 2.1). */
export const viewParts = {
  columns: "cols",
  widths: "widths",
  spacing: "spacing",
  groupBy: "group",
} as const;

/** The project page `/p/<short name>/issues`: the project the search looks in is in the path. */
const projectPath = /^\/p\/([^/]+)\/issues\/?$/u;

/** What an address says about a search and how its results are shown. */
export interface SearchState {
  readonly text: string;
  /** The short name of the project the list belongs to; none for the *Search* page. */
  readonly project?: string;
  readonly columns?: readonly string[];
  readonly widths?: Readonly<Record<string, number>>;
  readonly spacing?: SearchSpacing;
  readonly groupBy?: string;
  /** How it is shown (`list`, `board` …). */
  readonly view?: string;
}

/** Where a search state is written: the path and the parts after the question mark. */
export interface SearchLocation {
  readonly pathname: string;
  readonly search: Readonly<Record<string, string>>;
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
 * Reads a search from an address (the one place that does): the path says the project, `s` the
 * text, and the other parts how the results are shown. `search` is what the router read.
 */
export function readSearch(location: {
  readonly pathname?: string;
  readonly search: Readonly<Record<string, unknown>>;
}): SearchState {
  const raw = location.search;
  const address = readSearchAddress(raw);
  const project = projectPath.exec(location.pathname ?? "")?.[1];
  const columns = textOf(raw[viewParts.columns])
    ?.split(",")
    .filter((each) => each !== "");
  const widths = widthsOf(textOf(raw[viewParts.widths]));
  const spacing = textOf(raw[viewParts.spacing]);
  const groupBy = textOf(raw[viewParts.groupBy]);
  return {
    text: searchTextOf(address),
    ...(project === undefined ? {} : { project: decodeURIComponent(project) }),
    ...(columns === undefined || columns.length === 0 ? {} : { columns }),
    ...(widths === undefined ? {} : { widths }),
    ...(spacing === "comfortable" || spacing === "compact" ? { spacing } : {}),
    ...(groupBy === undefined || groupBy === "" ? {} : { groupBy }),
    ...(address.show === undefined ? {} : { view: address.show }),
  };
}

/** The address of a search state: the inverse of {@link readSearch}, and the only place that writes it. */
export function searchLocation(state: SearchState, searchPage = "/search"): SearchLocation {
  const search: Record<string, string> = {};
  if (state.text.trim() !== "") {
    search[searchParam] = state.text;
  }
  if (state.view !== undefined) {
    search[viewParam] = state.view;
  }
  if (state.columns !== undefined && state.columns.length > 0) {
    search[viewParts.columns] = state.columns.join(",");
  }
  const widths: string[] = [];
  for (const [field, width] of Object.entries(state.widths ?? {})) {
    widths.push(`${field}:${String(width)}`);
  }
  if (widths.length > 0) {
    search[viewParts.widths] = widths.join(",");
  }
  if (state.spacing !== undefined) {
    search[viewParts.spacing] = state.spacing;
  }
  if (state.groupBy !== undefined) {
    search[viewParts.groupBy] = state.groupBy;
  }
  const pathname =
    state.project === undefined ? searchPage : `/p/${encodeURIComponent(state.project)}/issues`;
  return { pathname, search };
}
