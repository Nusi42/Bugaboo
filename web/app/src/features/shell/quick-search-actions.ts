import { t } from "@bugaboo/messages";
import { containsEveryWord } from "@bugaboo/ui";

import { createRegistry } from "./registry.ts";
import type { QuickResult, QuickSearchAsker, QuickSearchSource } from "./quick-search-sources.ts";

/** Something quick search does instead of finding it: *Create issue*. */
export interface QuickAction {
  readonly id: string;
  /** What the person reads. */
  readonly title: string;
  readonly run: () => void;
}

const actions = createRegistry<QuickAction>();

/** Adds an action to quick search; returns the way to take it out again. */
export const addQuickAction = actions.add;

/** The actions that belong to the shell itself. */
function ownActions(asker: QuickSearchAsker): readonly QuickAction[] {
  return [{ id: "sign-out", title: t("nav.sign-out"), run: asker.signOut }];
}

/** The actions whose names hold every word typed. */
function findActions(query: string, asker: QuickSearchAsker): readonly QuickResult[] {
  return [...actions.list(), ...ownActions(asker)].flatMap((action) => {
    return containsEveryWord(action.title, query)
      ? [{ id: action.id, title: action.title, run: action.run }]
      : [];
  });
}

/** The source of the actions (M5-Q3): what a person can do from the keyboard, not only open. */
export const actionsSource: QuickSearchSource = {
  id: "actions",
  heading: t("search.actions"),
  search: findActions,
};
