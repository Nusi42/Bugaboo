import { useEffect, useRef, useState, type RefObject } from "react";

/** What the list was waiting for the last time it was looked at. */
export interface Moments {
  /** The server has yet to answer for the words typed. */
  readonly waiting: boolean;
  /** The person has stopped typing for the pause. */
  readonly quiet: boolean;
}

/** Whether a moment came: the typing pause ended (the rows of the app are there), or the answer landed. */
export function momentCame(was: Moments, now: Moments): boolean {
  return (was.waiting && !now.waiting) || (!was.quiet && now.quiet);
}

/**
 * Whether the focus is free to go to the first result: the box points at nothing, or at the result
 * that is first in the list as drawn. A box that points at another row, which includes the old first
 * row that a group landing on top has pushed down, is where the person (or an earlier moment) put
 * it, and stays.
 */
export function focusIsFree(active: string | null, first: string | undefined): boolean {
  return active === null || active === first;
}

/**
 * A number that goes up when the typing pause ends and when the answer of the server lands, unless
 * the person has moved the focus of the list: the list is made new with it, and a new list gives
 * its first result the virtual focus (the box names it with `aria-activedescendant`, and Enter
 * opens it). Whether the person moved the focus is read from where it is: the box points at
 * nothing, or at the result that is first in the list as drawn. The box clears the focus when a
 * letter is typed, so the pause and the answer give it again.
 *
 * `root` holds the box and the list.
 */
export function useFreshList(
  root: Readonly<RefObject<HTMLElement | null>>,
  waiting: boolean,
  quiet: boolean,
): number {
  const [fresh, setFresh] = useState(0);
  const was = useRef({ waiting, quiet });
  useEffect(() => {
    // The pause ended (the rows of the app are there, whatever the server does), or the answer landed.
    const landed = momentCame(was.current, { waiting, quiet });
    was.current = { waiting, quiet };
    // After the list is drawn, so the list as it is now is what is looked at.
    const timer = landed
      ? setTimeout(() => {
          const active =
            root.current?.querySelector("input")?.getAttribute("aria-activedescendant") ?? null;
          const first = root.current?.querySelector("[role='option']")?.id;
          if (focusIsFree(active, first)) {
            setFresh((n) => n + 1);
          }
        }, 0)
      : null;
    return () => {
      if (timer !== null) {
        clearTimeout(timer);
      }
    };
  }, [root, waiting, quiet]);
  return fresh;
}
