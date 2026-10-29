/**
 * How a list shows its results (UX design section 4.2): the columns, their widths, the spacing and
 * the field the rows are grouped by. The address holds only what a person changed, so a project's
 * default list (or a saved search) can change under an open page. The defaults come from the
 * built-in list, from the presentation of a saved search and from the person's own setting for the
 * spacing. Everything that turns an address into a view, or a view into an address or a saved
 * presentation, is here (ODR).
 */

import { settingDefaults, type Schema } from "@bugaboo/api-client";

import { currentWord, listWord, projectColumns, searchColumns } from "./list-columns.ts";
import { spacingOf, type SearchSpacing, type SearchState } from "./search-address.ts";

/**
 * What `group` says in the address when the person chose no grouping: the built-in list is grouped,
 * so a missing part cannot mean "none". A dash is no word of a field.
 */
export const noGrouping = "-";

/** The word of the field that the built-in lists group by: the status group (UX section 4.2). */
const builtInGroup = listWord.statusGroup;

/** How a list shows its results. */
export interface ListView {
  /** The search words of the columns, in order. */
  readonly columns: readonly string[];
  /** The width of the columns a person dragged, by search word. */
  readonly widths: Readonly<Record<string, number>>;
  readonly spacing: SearchSpacing;
  /** The search word of the field the rows are grouped by; none for no grouping. */
  readonly groupBy: string | null;
}

/** What the built-in list depends on. */
export interface BuiltInInput {
  /** The list is the *Search* page, over every project. */
  readonly everywhere: boolean;
  /** The person's own setting `appearance.spacing`. */
  readonly spacing: SearchSpacing;
}

/** The list a project or the *Search* page has before anybody changed anything. */
export function builtInView({ everywhere, spacing }: BuiltInInput): ListView {
  return {
    columns: everywhere ? searchColumns : projectColumns,
    widths: {},
    spacing,
    groupBy: builtInGroup,
  };
}

/** The view an address asks for: what it says, and the defaults for what it leaves out. */
export function viewOf(state: SearchState, defaults: ListView): ListView {
  const { groupBy } = state;
  return {
    columns: state.columns?.map(currentWord) ?? defaults.columns,
    widths: state.widths === undefined ? defaults.widths : wordsNow(state.widths),
    spacing: state.spacing ?? defaults.spacing,
    groupBy:
      groupBy === undefined
        ? defaults.groupBy
        : groupBy === noGrouping
          ? null
          : currentWord(groupBy),
  };
}

/** The widths with the words of the columns written the current way. */
function wordsNow(widths: Readonly<Record<string, number>>): Readonly<Record<string, number>> {
  const now: Record<string, number> = {};
  for (const word of Object.keys(widths)) {
    now[currentWord(word)] = widths[word] ?? 0;
  }
  return now;
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
  const { text, everything, project, saved, view: kind, open } = state;
  return {
    text,
    ...(everything === undefined ? {} : { everything }),
    ...(open === undefined ? {} : { open }),
    ...(project === undefined ? {} : { project }),
    ...(saved === undefined ? {} : { saved }),
    ...(kind === undefined ? {} : { view: kind }),
    ...(view.columns.join(",") === defaults.columns.join(",") ? {} : { columns: view.columns }),
    ...(sameWidths(view.widths, defaults.widths) ? {} : { widths: view.widths }),
    ...(view.spacing === defaults.spacing ? {} : { spacing: view.spacing }),
    ...(view.groupBy === defaults.groupBy ? {} : { groupBy: view.groupBy ?? noGrouping }),
  };
}

/** How a saved search keeps a view (QRY-06): the widths belong to their columns. */
export function presentationOf(view: ListView): Schema<"PresentationRef"> {
  return {
    view: "list",
    columns: view.columns.map((field) => ({ field, width: view.widths[field] ?? null })),
    spacing: view.spacing,
    group_by: view.groupBy,
  };
}

/**
 * The view a saved search keeps. A saved search without columns has the columns of the list it is
 * opened in (the project's), so `defaults` give them.
 */
export function viewFromPresentation(
  presentation: Schema<"PresentationRef">,
  defaults: ListView,
): ListView {
  const widths: Record<string, number> = {};
  for (const column of presentation.columns) {
    if (column.width !== null) {
      widths[currentWord(column.field)] = column.width;
    }
  }
  return {
    columns:
      presentation.columns.length === 0
        ? defaults.columns
        : presentation.columns.map((column) => currentWord(column.field)),
    widths,
    spacing: spacingOf(presentation.spacing) ?? settingDefaults.appearanceSpacing,
    groupBy: presentation.group_by === null ? null : currentWord(presentation.group_by),
  };
}
