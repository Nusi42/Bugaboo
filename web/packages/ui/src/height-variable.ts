import { useLayoutEffect, type RefObject } from "react";

import type { RunTimeVariable } from "./tokens.ts";
import { doNothing } from "./do-nothing.ts";
import { variableOf } from "./variables.ts";

/**
 * Gives the parent of an element the height of the element, as the browser draws it, in the run-time
 * variable `name` (THEME-ONLY: only a size the theme file marks `run-time="true"`): the rules of the
 * theme keep that much room for it, as the list keeps room at its end for the bar over its foot.
 */
export function useHeightOnParent(
  ref: Readonly<RefObject<HTMLElement | null>>,
  name: RunTimeVariable,
): void {
  useLayoutEffect(() => {
    const element = ref.current;
    const parent = element?.parentElement ?? null;
    if (element === null || parent === null) {
      return doNothing;
    }
    const note = (): void => {
      parent.style.setProperty(variableOf(name), `${String(element.offsetHeight)}px`);
    };
    note();
    const watch = new ResizeObserver(note);
    watch.observe(element);
    return () => {
      watch.disconnect();
      parent.style.setProperty(variableOf(name), "0px");
    };
  }, [ref, name]);
}
