import { useLayout } from "@bugaboo/ui";
import { useId, useMemo } from "react";

import { isOwnField, phoneColumns } from "../../shared/list-columns.ts";
import type { SearchState } from "../../shared/search-address.ts";
import type { ListView } from "../../shared/list-view.ts";
import { useNow } from "../../shared/use-now.ts";
import type { ListContextValue } from "./list-context.ts";
import { useOwnColumns } from "./own-columns.ts";

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
}

/** Whether a column or the grouping is a field of the organization, so its fields must be read. */
function needsOwn(view: ContextInput["view"]): boolean {
  return view.columns.some(isOwnField) || (view.groupBy !== null && isOwnField(view.groupBy));
}

/** What every row and group of the list shares: the columns it draws, the search, and who may tick. */
export function useListContextValue(input: ContextInput): ListContextValue {
  const phone = useLayout() === "phone";
  const { view } = input;
  const own = useOwnColumns(input.language, needsOwn(view));
  const listId = useId();
  const now = useNow();
  const { text, within, project, language, openIn, state, open, selection, onClear, report } =
    input;
  const columns = view.columns.join(",");
  const { groupBy } = view;
  // The same value while nothing it holds changed: every row and group reads it.
  return useMemo(
    () => ({
      listId,
      words: phone ? phoneColumns : columns.split(","),
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
      groupBy,
      onClear,
      report,
    }),
    [
      listId,
      phone,
      columns,
      groupBy,
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
