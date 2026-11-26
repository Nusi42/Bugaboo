import type { SearchLanguage } from "@bugaboo/wasm-search";

import type { ListView } from "../../shared/list-view.ts";
import type { SearchFieldsAnswer } from "../search/index.ts";

/** What a calendar is of: a search, where it looks, and the view. */
export interface CalendarInput {
  readonly text: string;
  /** The ID of the project it looks in; none looks in every project the person sees. */
  readonly within: string | undefined;
  readonly view: ListView;
  readonly language: SearchLanguage;
  /** The fields the person may read, with their names. */
  readonly fields: Pick<SearchFieldsAnswer, "names">;
  /** Whether the screen is a phone: the list of the days to come, not the grid. */
  readonly phone: boolean;
}
