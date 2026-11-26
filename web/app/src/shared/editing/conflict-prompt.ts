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

/**
 * Shows a prompt over the page, or takes it away (`null`). A new prompt in place of one that was not
 * answered answers that one first with *Keep their change*, so no bar keeps showing dates that were
 * never saved.
 */
export function askAboutConflict(prompt: ConflictPrompt | null): void {
  const before = asked.snapshot();
  if (before !== null && prompt !== null && before !== prompt) {
    before.keepTheirs();
  }
  asked.replace(prompt);
}

/** A screen that asks over the page: what it asks once it is gone is never shown on another page. */
export interface ConflictAsker {
  /** Shows a prompt, unless the screen is gone (its words may come after it went). */
  readonly ask: (prompt: ConflictPrompt) => void;
  /** The screen is there (again). */
  readonly stay: () => void;
  /** The screen goes: its prompt goes with it. */
  readonly leave: () => void;
}

/** A new asker, for one screen. */
export function conflictAsker(): ConflictAsker {
  let gone = false;
  return {
    ask: (prompt) => {
      if (!gone) {
        askAboutConflict(prompt);
      }
    },
    stay: () => {
      gone = false;
    },
    leave: () => {
      gone = true;
      askAboutConflict(null);
    },
  };
}

/** The prompt over the page, while there is one. */
export function useAskedConflict(): ConflictPrompt | null {
  return useSyncExternalStore(asked.subscribe, asked.snapshot);
}

/**
 * Asks over the page which of two changes of one thing stays, for a screen that saves a change at once (a
 * bar of the timeline, an issue of the calendar): *Keep their change* or *Use mine*. The words of the
 * prompt come with the answer of the server, so the prompt may show a moment later; it shows only while
 * the screen is there (`asker`).
 */
export function askAboutClash(
  conflict: Conflict,
  answers: Pick<ConflictPrompt, "keepTheirs" | "useMine">,
  asker: ConflictAsker,
): void {
  void conflictPromptOf(conflict, answers).then(asker.ask);
}
