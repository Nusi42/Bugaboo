/**
 * What Simple search does with the tree of a search (ADR-0005): it lists the filters, takes one
 * away, changes one or adds one, and groups some under *Any of*. It never writes text: the printer
 * of the search language does (`SearchLanguage.print`), so there is one definition of the syntax,
 * and the new tree is read again from that text to get its places.
 */
import type { Expr, Search, Span } from "@bugaboo/wasm-search";

/** A place in no text: the printer does not read places, and the text it writes is parsed again. */
export const nowhere: Span = { start: 0, end: 0 };

/** A filter on a field. */
export type FieldFilter = Extract<Expr, { type: "field" }>;

/** What is asked of a field. */
export type FieldTest = FieldFilter["test"];

/**
 * Where a filter stands: its place among the filters of the search, and its place among the
 * members of an *Any of* when it is inside one.
 */
export interface FilterPlace {
  readonly index: number;
  readonly member?: number | undefined;
}

/** The filters Simple search shows as buttons: the members of an `and`, or the one filter. */
export function filtersOf(search: Search): readonly Expr[] {
  const { filter } = search;
  if (filter === null) {
    return [];
  }
  return filter.type === "and" ? filter.items : [filter];
}

/** The members of an *Any of*: nothing for a filter that is not one. */
export function membersOf(filter: Expr): readonly Expr[] {
  return filter.type === "or" ? filter.items : [];
}

/** The filter at `place`, if there is one. */
export function filterAt(search: Search, place: FilterPlace): Expr | undefined {
  const filter = filtersOf(search)[place.index];
  return place.member === undefined || filter === undefined
    ? filter
    : membersOf(filter)[place.member];
}

/** `and` of at least two filters, the filter itself for one, nothing for none. */
function joinedAnd(filters: readonly Expr[]): Expr | null {
  // A tree has no `and` inside an `and`: a group of filters that arrives is spread out.
  const items = filters.flatMap((each) => (each.type === "and" ? each.items : [each]));
  const [first, second] = items;
  if (first === undefined) {
    return null;
  }
  return second === undefined ? first : { type: "and", items, span: nowhere };
}

/** `or` of at least two filters, the filter itself for one, nothing for none. */
function joinedOr(members: readonly Expr[]): Expr | null {
  const items = members.flatMap((each) => (each.type === "or" ? each.items : [each]));
  const [first, second] = items;
  if (first === undefined) {
    return null;
  }
  return second === undefined ? first : { type: "or", items, span: nowhere };
}

/** The search with these filters, the order of the results as it was. */
export function withFilters(search: Search, filters: readonly Expr[]): Search {
  return { ...search, filter: joinedAnd(filters) };
}

/** The list without the item at `index`. */
function without<Item>(items: readonly Item[], index: number): Item[] {
  return items.filter((_, at) => at !== index);
}

/** The list with `item` at `index`. */
function replaced<Item>(items: readonly Item[], index: number, item: Item): Item[] {
  return items.map((each, at) => (at === index ? item : each));
}

/** The filters with the one at `place` changed to `filter`, or taken away when it is `null`. */
function editedAt(
  filters: readonly Expr[],
  place: FilterPlace,
  filter: Expr | null,
): readonly Expr[] {
  const { index, member } = place;
  const current = filters[index];
  if (current === undefined) {
    return filters;
  }
  if (member === undefined) {
    return filter === null ? without(filters, index) : replaced(filters, index, filter);
  }
  const members = membersOf(current);
  const changed = filter === null ? without(members, member) : replaced(members, member, filter);
  const group = joinedOr(changed);
  return group === null ? without(filters, index) : replaced(filters, index, group);
}

/** The search with the filter at `place` changed to `filter`. */
export function changeFilter(search: Search, place: FilterPlace, filter: Expr): Search {
  return withFilters(search, editedAt(filtersOf(search), place, filter));
}

/** The search without the filter at `place` (the ✕ of its button). */
export function removeFilter(search: Search, place: FilterPlace): Search {
  return withFilters(search, editedAt(filtersOf(search), place, null));
}

/** The search with one more filter at the end. */
export function addFilter(search: Search, filter: Expr): Search {
  return withFilters(search, [...filtersOf(search), filter]);
}

/** The search with one more member in the *Any of* at `index`. */
export function addMember(search: Search, index: number, filter: Expr): Search {
  const filters = filtersOf(search);
  const group = filters[index];
  const joined = group === undefined ? null : joinedOr([...membersOf(group), filter]);
  return joined === null ? search : withFilters(search, replaced(filters, index, joined));
}

/** The search with the filter at `index` and a new one together as an *Any of*. */
export function groupAsAnyOf(search: Search, index: number, other: Expr): Search {
  const filters = filtersOf(search);
  const first = filters[index];
  const joined = first === undefined ? null : joinedOr([first, other]);
  return joined === null ? search : withFilters(search, replaced(filters, index, joined));
}

/** A filter on a field, built to be printed: `word` is the word of the field. */
export function fieldFilter(word: string, test: FieldTest): Expr {
  return { type: "field", field: { word, span: nowhere }, test, span: nowhere };
}

/**
 * The search with the filters of an example added to its own; an example that orders the results
 * replaces the order (a search has one).
 */
export function withExample(search: Search, example: Search): Search {
  return {
    ...withFilters(search, [...filtersOf(search), ...filtersOf(example)]),
    sort: example.sort.length > 0 ? example.sort : search.sort,
  };
}
