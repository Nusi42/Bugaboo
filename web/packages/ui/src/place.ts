import { useLayoutEffect, type RefObject } from "react";

import { runTimeVariables, type RunTimeVariable } from "./tokens.ts";
import { variableOf } from "./variables.ts";

/**
 * Where a thing stands on a drawing of days (the timeline, UX §4.7): numbers only, by the tokens the
 * base theme file marks `run-time="true"` (`timeline-start`, `timeline-length` …). The rules of the
 * theme file turn them into places, so a color or a spacing is never set from code (THEME-ONLY).
 */
export type Place = Partial<Readonly<Record<RunTimeVariable, number>>>;

/**
 * Gives the element its place each time it is drawn: the browser draws again only what a changed
 * number moves, so a bar that is dragged moves without the rest of the chart being touched.
 */
export function usePlace(
  ref: Readonly<RefObject<HTMLElement | SVGElement | null>>,
  place: Place,
): void {
  useLayoutEffect(() => {
    const element = ref.current;
    if (element === null) {
      return;
    }
    for (const name of runTimeVariables) {
      const value = place[name];
      if (value !== undefined) {
        element.style.setProperty(variableOf(name), String(value));
      }
    }
  });
}
