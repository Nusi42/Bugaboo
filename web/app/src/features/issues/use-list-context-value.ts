import { useLayout } from "@bugaboo/ui";
import { useId, useMemo } from "react";

import type { SearchState } from "../../shared/search-address.ts";
import { useNow } from "../../shared/use-now.ts";
import type { ListContextValue } from "./list-context.ts";
import { useListRead, type ReadInput } from "./use-list-read.ts";

/** What the context of a list is made of. */
export interface ContextInput extends ReadInput {
  readonly openIn: ListContextValue["openIn"];
  readonly state: SearchState | undefined;
  readonly open: string | undefined;
  readonly selection: ListContextValue["selection"];
  readonly onClear: () => void;
  readonly report: ListContextValue["report"];
}

/**
 * What every row and group of the list shares: the columns it draws, the search, and who may tick. It is the same
 * value while nothing it holds changed: every row and group reads it.
 */
export function useListContextValue(input: ContextInput): ListContextValue {
  const phone = useLayout() === "phone";
  const listId = useId();
  const now = useNow();
  const { view, text, within, project, language, openIn, state, open, selection, onClear, report } =
    input;
  const { words, hidden, failure, own } = useListRead(input);
  return useMemo(
    () => ({
      listId,
      words,
      hidden,
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
      hidden,
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
