import type { SearchLanguage } from "@bugaboo/wasm-search";

import type { ListView } from "../../shared/list-view.ts";
import type { HistoryStep } from "../../shared/search-address.ts";
import type { KeptIn } from "../../shared/use-list-view.ts";
import type { SearchFieldsAnswer } from "../search/index.ts";

export interface BoardResultsProps {
  /** The canonical text of the search. */
  readonly text: string;
  readonly onText: (text: string, step: HistoryStep) => void;
  /** The ID of the project it looks in; none looks in every project the person sees. */
  readonly within: string | undefined;
  readonly view: ListView;
  readonly onView: (view: ListView) => void;
  /** The saved search of the person's own that the page shows, where the options of the board are kept. */
  readonly keptIn: KeptIn | undefined;
  readonly fields: SearchFieldsAnswer;
  readonly language: SearchLanguage;
}
