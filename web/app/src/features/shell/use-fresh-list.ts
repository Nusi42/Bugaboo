import { useEffect, useRef, useState, type RefObject } from "react";

/**
 * A number that goes up when the typing pause ends and when the answer of the server lands, unless the person has moved the focus
 * of the list: the list is made new with it, and a new list gives its first result the virtual
 * focus (the box names it with `aria-activedescendant`, and Enter opens it). Whether the person
 * moved the focus is read from where it is: the box points at nothing, or at the result that is
 * first in the list as drawn. The box clears the focus when a letter is typed, so the next answer gives it again.
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
    const landed = (was.current.waiting && !waiting) || (!was.current.quiet && quiet);
    was.current = { waiting, quiet };
    // After the answer is drawn, so the list as it is now is what is looked at.
    const timer = landed
      ? setTimeout(() => {
          const active =
            root.current?.querySelector("input")?.getAttribute("aria-activedescendant") ?? null;
          const first = root.current?.querySelector("[role='option']")?.id;
          if (active === null || active === first) {
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
