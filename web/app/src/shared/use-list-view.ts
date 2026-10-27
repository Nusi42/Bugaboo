import type { Schema } from "@bugaboo/api-client";

import { appearanceOf } from "./appearance-setting.ts";
import { builtInView, viewFromPresentation, viewOf, withView, type ListView } from "./list-view.ts";
import { historyStep, type HistoryStep, type SearchState } from "./search-address.ts";
import { useMySettings } from "./use-my-settings.ts";

/** What a page that shows the results of a search gives its list. */
export interface ResultsInput {
  /** The canonical text of the search. */
  readonly text: string;
  readonly onText: (text: string, step: HistoryStep) => void;
  readonly view: ListView;
  /** The view the project's list, or the saved search, has: where the columns go back to. */
  readonly defaults: ListView;
  readonly onView: (view: ListView) => void;
  /** The text in the search box has a mistake: the results stay, dimmed. */
  readonly stale: boolean;
}

/** What the view of a list is made from. */
export interface ViewSource {
  /** The list is the *Search* page, over every project. */
  readonly everywhere: boolean;
  /** How a saved search, or the saved search that is the project's list, shows its results. */
  readonly presentation?: Schema<"PresentationRef"> | undefined;
}

/** The view a list has: the address where it says anything, then the saved search, then the built-in list and the person's spacing. */
export function useListView(
  state: SearchState,
  onState: (state: SearchState, step: HistoryStep) => void,
  { everywhere, presentation }: ViewSource,
): Pick<ResultsInput, "view" | "defaults" | "onView"> {
  const spacing = appearanceOf(useMySettings()).spacing;
  const builtIn = builtInView({ everywhere, spacing });
  const defaults =
    presentation === undefined ? builtIn : viewFromPresentation(presentation, builtIn);
  return {
    view: viewOf(state, defaults),
    defaults,
    onView: (view) => {
      onState(withView(state, view, defaults), historyStep.replace);
    },
  };
}
