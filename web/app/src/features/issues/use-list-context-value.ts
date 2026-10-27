import { useLayout } from "@bugaboo/ui";
import type { SearchLanguage } from "@bugaboo/wasm-search";

import { isOwnField, phoneColumns } from "../../shared/list-columns.ts";
import type { SearchState } from "../../shared/search-address.ts";
import type { ListView } from "../../shared/list-view.ts";
import { useNow } from "../../shared/use-now.ts";
import type { IssueShow } from "./issue-show.ts";
import type { ListContextValue } from "./list-context.ts";
import { useOwnColumns } from "./own-columns.ts";
import type { IssueSelection } from "./use-selection.ts";

/** What the context of a list is made of. */
export interface ContextInput {
  readonly text: string;
  readonly within: string | undefined;
  readonly project: string | undefined;
  readonly view: ListView;
  readonly language: SearchLanguage;
  readonly openIn: IssueShow;
  readonly state: SearchState | undefined;
  readonly open: string | undefined;
  readonly selection: IssueSelection;
  readonly onClear: () => void;
  readonly report: ListContextValue["report"];
}

/** What every row and group of the list shares: the columns it draws, the search, and who may tick. */
export function useListContextValue(input: ContextInput): ListContextValue {
  const phone = useLayout() === "phone";
  const { view } = input;
  const own = useOwnColumns(
    input.language,
    view.columns.some(isOwnField) || (view.groupBy !== null && isOwnField(view.groupBy)),
  );
  return {
    words: phone ? phoneColumns : view.columns,
    phone,
    ticks: input.project !== undefined,
    openIn: input.openIn,
    state: input.state,
    open: input.open,
    selection: input.selection,
    project: input.project,
    now: useNow(),
    own,
    search: { text: input.text, within: input.within },
    language: input.language,
    groupBy: view.groupBy,
    onClear: input.onClear,
    report: input.report,
  };
}
