import type { Schema } from "@bugaboo/api-client";
import { useState } from "react";

import type { EditLock } from "../../shared/editing/index.ts";
import type { NamedPair } from "./model.ts";
import { refusedPairsOf } from "./refusals.ts";
import { useSaveTheme, valuesBody } from "./themes.ts";
import type { Editing } from "./use-theme-editing.ts";

/** What the person is asked or told before a save goes to the server. */
export type SaveQuestion =
  /** A personal theme with pairs that are hard to read: *Save anyway?* */
  | "ask"
  /** A theme for everyone with pairs that are hard to read: it cannot be saved until they are fixed. */
  | "refused"
  | null;

/** The save of the editor and what it asks first. */
export interface Saving {
  readonly question: SaveQuestion;
  readonly close: () => void;
  /** *Save*: asks first when a pair is hard to read, refuses a theme for everyone. */
  readonly save: () => void;
  /** Sends the save as it is (*Save anyway* of a personal theme). */
  readonly send: () => void;
  readonly pending: boolean;
  readonly saved: boolean;
  /** Why the save did not work, unless it is the refusal of the pairs (shown with them). */
  readonly error: unknown;
  /** The pairs to fix before a theme for everyone can be saved, from the editor or the server. */
  readonly refused: readonly NamedPair[];
}

/**
 * The save of an own theme (UX §5.6): a theme for everyone with a pair that is hard to read is not
 * sent, and the pairs are named; a personal one asks *Save anyway?* first. The server checks again
 * and names the pairs itself when it refuses. The save goes under the lock the person holds.
 */
export function useSaving(
  detail: Schema<"ThemeDetailBody">,
  editing: Editing,
  lock: EditLock,
): Saving {
  const { theme } = detail;
  const [question, setQuestion] = useState<SaveQuestion>(null);
  const put = useSaveTheme();
  const fromServer = refusedPairsOf(put.error);
  const send = () => {
    setQuestion(null);
    if (lock.state.phase === "mine") {
      const body = {
        version: theme.version,
        lock_token: lock.state.token,
        name: editing.name.trim(),
        values: valuesBody(editing.values),
      };
      put.mutate(
        { params: { path: { theme: theme.id } }, body },
        {
          onSettled: (_saved: unknown, error: unknown) => {
            lock.settled(error);
          },
        },
      );
    }
  };
  return {
    question,
    close: () => {
      setQuestion(null);
    },
    save: () => {
      const kind = theme.kind === "for-everyone" ? "refused" : "ask";
      if (editing.hard.length === 0) {
        send();
      } else {
        setQuestion(kind);
      }
    },
    send,
    pending: put.isPending,
    saved: put.isSuccess,
    error: fromServer.length > 0 ? null : put.error,
    refused: fromServer.length > 0 ? fromServer : question === "refused" ? editing.hard : [],
  };
}
