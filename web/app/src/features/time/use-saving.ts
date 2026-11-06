import { doNothing, type SaveState } from "@bugaboo/ui";
import { useCallback, useRef, useState } from "react";

/** How a save went: it worked, or the error the server (or the network) answered with. */
export type Saved = { readonly ok: true } | { readonly ok: false; readonly error: unknown };

/** Starts one save and says how it went; it never throws. */
async function attempt(work: () => Promise<void>): Promise<Saved> {
  try {
    await work();
    return { ok: true };
  } catch (error) {
    return { ok: false, error };
  }
}

/**
 * *Saving…*, then *Saved* or *Not saved yet* for the changes of the page (UX §6). What is typed is
 * handled one after the other, in the order it was typed (`inLine`), because a change of a day is
 * made on the version the one before left.
 */
export function useSaving() {
  const [state, setState] = useState<SaveState>("idle");
  const queue = useRef<Promise<unknown>>(Promise.resolve());
  const inLine = useCallback(<Result>(job: () => Promise<Result>): Promise<Result> => {
    const turn = queue.current.then(job);
    queue.current = turn.then(doNothing, doNothing);
    return turn;
  }, []);
  /** Saves at once, in the turn that is running. */
  const saveNow = useCallback(async (work: () => Promise<void>): Promise<Saved> => {
    setState("saving");
    const saved = await attempt(work);
    setState(saved.ok ? "saved" : "not-saved");
    return saved;
  }, []);
  const run = useCallback(
    (work: () => Promise<void>) => inLine(() => saveNow(work)),
    [inLine, saveNow],
  );
  return { state, run, inLine, saveNow };
}
