import type { SearchLanguage } from "@bugaboo/wasm-search";
import { useLayout } from "@bugaboo/ui";
import { useMemo } from "react";

import { isOwnField, phoneColumns } from "../../shared/list-columns.ts";
import type { ListView } from "../../shared/list-view.ts";
import { useSameWidths } from "./use-same-widths.ts";
import { columnsThatFit } from "./fit-columns.ts";
import { useFoundGroups, useFoundIssues, type FoundSearch } from "./found-issues.ts";
import { useOwnColumns, type OwnColumns } from "./own-columns.ts";

/** What the list reads: the words of its columns, the own fields they need, and why the search was refused. */
export interface ListRead {
  /** The search words of the columns, in order (a phone draws its own few). */
  readonly words: readonly string[];
  /** How many of the columns the person chose give way for lack of room (none on a phone, which draws its own). */
  readonly hidden: number;
  /** The own fields a column or the grouping is. */
  readonly own: OwnColumns;
  /**
   * Why the search of the list was refused, so that there are no results to show, not even an
   * earlier list (the address itself has a mistake); none while there is a list. The one place that
   * decides it: the list context holds the answer, and the head, the count line and the results
   * read it there.
   */
  readonly failure: Error | null;
}

/** Whether a column or the grouping is a field of the organization, so its fields must be read. */
function needsOwn(view: ListView): boolean {
  return view.columns.some(isOwnField) || (view.groupBy !== null && isOwnField(view.groupBy));
}

/** What reading the list needs: the search, how it is shown, and the room it has. */
export interface ReadInput {
  readonly text: string;
  readonly within: string | undefined;
  readonly view: ListView;
  readonly language: SearchLanguage;
  /** The width the list has, in pixels; none before it is measured. */
  readonly room: number | undefined;
  /** The widths dragged now, by search word; the kept ones are in the view. */
  readonly dragged: Readonly<Record<string, number>>;
  /** The characters of the longest readable ID drawn. */
  readonly keyChars: number;
  /** The project whose list it is; the rows of a list of a project have a box to tick. */
  readonly project: string | undefined;
}

/** Reads what the list shows: the same queries as its rows and groups, which are the same entries of the server-state cache. */
export function useListRead(input: ReadInput): ListRead {
  const { text, within, view, language, room, project, dragged, keyChars } = input;
  const search: FoundSearch = { text, within };
  const ticks = project !== undefined;
  // A phone has its own few columns; where the list has little room (beside the side panel, or with Customize open) the columns of fields give way.
  const phone = useLayout() === "phone";
  const columns = view.columns.join(",");
  const widths = useSameWidths({ ...view.widths, ...dragged });
  const { words, hidden } = useMemo(() => {
    if (phone) {
      return { words: phoneColumns, hidden: 0 };
    }
    const chosen = columns.split(",");
    const fit = columnsThatFit(chosen, { room, ticks, widths, keyChars });
    return { words: fit, hidden: chosen.length - fit.length };
  }, [phone, room, ticks, columns, widths, keyChars]);

  const own = useOwnColumns(language, needsOwn(view));
  const rows = useFoundIssues(search, words, view.groupBy === null);
  const groups = useFoundGroups(search, view.groupBy);
  const read = view.groupBy === null ? rows : groups;
  return { words, hidden, own, failure: read.isError ? read.error : null };
}
