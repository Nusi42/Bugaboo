import type { Schema } from "@bugaboo/api-client";
import { useState } from "react";

import type { EditLock } from "../../shared/editing/index.ts";
import { focusReadability } from "./focus-color.ts";
import { useSaveTheme, valuesBody } from "./themes.ts";
import type { Editing } from "./use-theme-editing.ts";

/** What the person is asked or told before a save goes to the server. */
export type SaveQuestion =
  /** A personal theme with colors that are hard to read: *Save this theme?* */
  "ask" | null;

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
  /** Why the save did not work. */
  readonly error: unknown;
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
      if (editing.hardCount === 0) {
        send();
      } else if (theme.kind === "for-everyone") {
        // It is said once, on top, where the colors to change are named: the cursor goes there.
        editing.refuse();
        window.requestAnimationFrame(focusReadability);
      } else {
        setQuestion("ask");
      }
    },
    send,
    pending: put.isPending,
    saved: put.isSuccess,
    error: put.error,
  };
}
