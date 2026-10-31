import type { Schema } from "@bugaboo/api-client";

import {
  builtInView,
  viewFromPresentation,
  viewInWords,
  viewOf,
  withView,
  type ListView,
} from "./list-view.ts";
import { historyStep, type HistoryStep, type SearchState } from "./search-address.ts";

/** What a page that shows the results of a search gives its list. */
export interface ResultsInput {
  /** The canonical text of the search. */
  readonly text: string;
  readonly onText: (text: string, step: HistoryStep) => void;
  readonly view: ListView;
  /** The view the project's list, or the saved search, has: where the columns go back to. */
  readonly defaults: ListView;
  readonly onView: (view: ListView) => void;
  /** The text in the search box has a mistake: the results stay, with a dashed edge and a note. */
  readonly stale: boolean;
}

/** What the view of a list is made from. */
export interface ViewSource {
  /** The list is the *Search* page, over every project. */
  readonly everywhere: boolean;
  /** How a saved search, or the saved search that is the project's list, shows its results. */
  readonly presentation?: Schema<"PresentationRef"> | undefined;
  /** The word of a field as the person may read it now, for a list saved with the old words; none before the fields are there. */
  readonly word?: ((word: string) => string) | undefined;
}

/** The view a list has: the address where it says anything, then the saved search, then the built-in list. */
export function useListView(
  state: SearchState,
  onState: (state: SearchState, step: HistoryStep) => void,
  { everywhere, presentation, word }: ViewSource,
): Pick<ResultsInput, "view" | "defaults" | "onView"> {
  const builtIn = builtInView({ everywhere });
  const saved = presentation === undefined ? builtIn : viewFromPresentation(presentation, builtIn);
  // The words are mapped once, in the defaults and in the view, so a view set back by hand is the
  // defaults again and nothing looks changed.
  const defaults = word === undefined ? saved : viewInWords(saved, word);
  const own = viewOf(state, defaults);
  return {
    view: word === undefined ? own : viewInWords(own, word),
    defaults,
    onView: (view) => {
      onState(withView(state, view, defaults), historyStep.replace);
    },
  };
}

/** Whether the address changes any part of how the results are shown, so that it differs from the defaults. */
export function viewIsChanged(state: SearchState): boolean {
  return state.columns !== undefined || state.widths !== undefined || state.groupBy !== undefined;
}

/** The state with nothing unsaved: no text of its own and no change of the view (what a saved search has once it was saved). */
export function withoutChanges(state: SearchState): SearchState {
  const { text: _t, everything: _e, columns: _c, widths: _w, groupBy: _g, ...rest } = state;
  return { ...rest, text: "" };
}
