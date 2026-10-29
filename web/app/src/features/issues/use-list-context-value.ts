import { useLayout } from "@bugaboo/ui";
import { useId, useMemo } from "react";

import type { SearchState } from "../../shared/search-address.ts";
import type { ListView } from "../../shared/list-view.ts";
import { useNow } from "../../shared/use-now.ts";
import type { ListContextValue } from "./list-context.ts";
import { useListRead } from "./use-list-read.ts";

/** What the context of a list is made of. */
export interface ContextInput {
  readonly text: string;
  readonly within: string | undefined;
  readonly project: string | undefined;
  readonly view: ListView;
  readonly language: ListContextValue["language"];
  readonly openIn: ListContextValue["openIn"];
  readonly state: SearchState | undefined;
  readonly open: string | undefined;
  readonly selection: ListContextValue["selection"];
  readonly onClear: () => void;
  readonly report: ListContextValue["report"];
  /** The width the list has, in pixels; none before it is measured. */
  readonly room: number | undefined;
}

/** What every row and group of the list shares: the columns it draws, the search, and who may tick. */
export function useListContextValue(input: ContextInput): ListContextValue {
  const phone = useLayout() === "phone";
  const listId = useId();
  const now = useNow();
  const { view, text, within, project, language, openIn, state, open, selection } = input;
  const { onClear, report } = input;
  const { words, failure, own } = useListRead(input);
  // The same value while nothing it holds changed: every row and group reads it.
  return useMemo(
    () => ({
      listId,
      words,
      phone,
      ticks: project !== undefined,
      openIn,
      state,
      open,
      selection,
      project,
      now,
      own,
      search: { text, within },
      language,
      groupBy: view.groupBy,
      failure,
      onClear,
      report,
    }),
    [
      listId,
      phone,
      words,
      failure,
      view.groupBy,
      project,
      openIn,
      state,
      open,
      selection,
      now,
      own,
      text,
      within,
      language,
      onClear,
      report,
    ],
  );
}
