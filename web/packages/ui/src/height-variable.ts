import { useLayoutEffect, type RefObject } from "react";

import type { RunTimeVariable } from "./tokens.ts";
import { doNothing } from "./do-nothing.ts";
import { variableOf } from "./variables.ts";

/**
 * Gives `holder` the height of an element, as the browser draws it, in the run-time variable `name` (THEME-ONLY: only
 * a size the theme file marks `run-time="true"`): the rules of the theme keep that much room for it inside the holder.
 * `onHeight` hears the same height, for what lays itself out in code (a list drawn only where it is on screen).
 */
export function useHeightOn(
  ref: Readonly<RefObject<HTMLElement | null>>,
  holder: (element: HTMLElement) => HTMLElement | null,
  name: RunTimeVariable,
  onHeight: (height: number) => void = doNothing,
): void {
  useLayoutEffect(() => {
    const element = ref.current;
    const target = element === null ? null : holder(element);
    if (element === null || target === null) {
      return doNothing;
    }
    const note = (): void => {
      target.style.setProperty(variableOf(name), `${String(element.offsetHeight)}px`);
      onHeight(element.offsetHeight);
    };
    note();
    const watch = new ResizeObserver(note);
    watch.observe(element);
    return () => {
      watch.disconnect();
      target.style.setProperty(variableOf(name), "0px");
    };
  }, [ref, holder, name, onHeight]);
}

/** The parent of an element. */
function parentOf(element: HTMLElement): HTMLElement | null {
  return element.parentElement;
}

/**
 * Gives the parent of an element its height, as the list keeps room at its end for the bar over its foot
 * ({@link useHeightOn}).
 */
export function useHeightOnParent(
  ref: Readonly<RefObject<HTMLElement | null>>,
  name: RunTimeVariable,
): void {
  useHeightOn(ref, parentOf, name);
}
