import type { Conflict } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useSyncExternalStore } from "react";

import { createStore } from "../store.ts";
import { changedByName, conflictSentenceWhenLoaded } from "./conflict-text.ts";

/**
 * Somebody changed it first (UX §4.4, M3-Q5 d): the sentence built from the structured answer of
 * the server, and the two answers. The issue page and the timeline ask with the same words.
 */
export interface ConflictPrompt {
  readonly sentence: string;
  /** The words of *Keep Ben's change*. */
  readonly theirs: string;
  readonly keepTheirs: () => void;
  readonly useMine: () => void;
}

/** *Keep Ben's change*; for a person who is not shown by name, *Keep their change*. */
export function keepTheirsText(conflict: Conflict): string {
  return conflict.changed_by?.shown === "name"
    ? t("issues.keep-theirs", { name: changedByName(conflict) })
    : t("issues.keep-theirs-unnamed");
}

/** The words of the prompt of a change that came second: what happened, and *Keep Ben's change*. */
export async function conflictWordsOf(
  conflict: Conflict,
): Promise<Pick<ConflictPrompt, "sentence" | "theirs">> {
  return {
    sentence: await conflictSentenceWhenLoaded(conflict, Date.now()),
    theirs: keepTheirsText(conflict),
  };
}

/** The prompt of a change that came second: what happened, and what each answer does. */
export async function conflictPromptOf(
  conflict: Conflict,
  answers: Pick<ConflictPrompt, "keepTheirs" | "useMine">,
): Promise<ConflictPrompt> {
  return { ...(await conflictWordsOf(conflict)), ...answers };
}

/**
 * The prompt of a screen whose changes have no place of their own to ask in (a bar of the timeline):
 * it stands where the offers to undo stand (`UndoHost`) until it is answered.
 */
const asked = createStore<ConflictPrompt | null>(null);

/** Shows a prompt over the page, in place of one that was not answered. */
export function askAboutConflict(prompt: ConflictPrompt | null): void {
  asked.replace(prompt);
}

/** The prompt over the page, while there is one. */
export function useAskedConflict(): ConflictPrompt | null {
  return useSyncExternalStore(asked.subscribe, asked.snapshot);
}
