import { useLayout } from "@bugaboo/ui";
import type { SearchLanguage } from "@bugaboo/wasm-search";

import { cellOf } from "../../shared/list-columns.ts";
import type { ListView } from "../../shared/list-view.ts";
import { useNow } from "../../shared/use-now.ts";
import type { IssueShow } from "./issue-show.ts";
import type { ListContextValue } from "./list-context.ts";
import { useOwnColumns } from "./own-columns.ts";
import type { IssueSelection } from "./use-selection.ts";

/** What a phone draws in each card (UX section 4.2): columns are not offered there. */
const phoneColumns: readonly string[] = [
  "status",
  "id",
  "priority",
  "title",
  "assignedTo",
  "changed",
];

/** What the context of a list is made of. */
export interface ContextInput {
  readonly text: string;
  readonly within: string | undefined;
  readonly project: string | undefined;
  readonly view: ListView;
  readonly language: SearchLanguage;
  /** The words of the choices of the field the list is grouped by. */
  readonly choices: readonly string[];
  readonly openIn: IssueShow;
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
    view.columns.some((word) => cellOf(word) === "value"),
  );
  return {
    words: phone ? phoneColumns : view.columns,
    phone,
    ticks: input.project !== undefined,
    openIn: input.openIn,
    open: input.open,
    selection: input.selection,
    project: input.project,
    now: useNow(),
    own,
    search: { text: input.text, within: input.within },
    language: input.language,
    choices: input.choices,
    groupBy: view.groupBy,
    onClear: input.onClear,
    report: input.report,
  };
}
