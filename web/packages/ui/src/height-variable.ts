import { useLayoutEffect, useMemo, type RefObject } from "react";

import type { RunTimeVariable } from "./tokens.ts";
import { doNothing } from "./do-nothing.ts";
import { variableOf } from "./variables.ts";

/** A size of an element that the theme reads at run time, and how the browser's drawing of the element gives it. */
export interface RunTimeSize {
  readonly name: RunTimeVariable;
  readonly of: (element: HTMLElement, holder: HTMLElement) => number;
}

/**
 * Gives `holder` sizes of an element, as the browser draws it, in run-time variables (THEME-ONLY: only a size the theme
 * file marks `run-time="true"`), measured again whenever the element changes its size: the rules of the theme keep that
 * much room for it inside the holder. `onSizes` hears the same sizes, in their order, for what lays itself out in code
 * (a list drawn only where it is on screen), so the theme and the code read one measure. `sizes` does not change.
 */
export function useSizesOn(
  ref: Readonly<RefObject<HTMLElement | null>>,
  holder: (element: HTMLElement) => HTMLElement | null,
  sizes: readonly RunTimeSize[],
  onSizes: (sizes: readonly number[]) => void = doNothing,
): void {
  useLayoutEffect(() => {
    const element = ref.current;
    const target = element === null ? null : holder(element);
    if (element === null || target === null) {
      return doNothing;
    }
    const note = (): void => {
      const measured = sizes.map((size) => size.of(element, target));
      sizes.forEach((size, at) => {
        target.style.setProperty(variableOf(size.name), `${String(measured[at] ?? 0)}px`);
      });
      onSizes(measured);
    };
    note();
    const watch = new ResizeObserver(note);
    watch.observe(element);
    return () => {
      watch.disconnect();
      for (const size of sizes) {
        target.style.setProperty(variableOf(size.name), "0px");
      }
    };
  }, [ref, holder, sizes, onSizes]);
}

/** The parent of an element. */
function parentOf(element: HTMLElement): HTMLElement | null {
  return element.parentElement;
}

/** The height of an element as it is drawn. */
function heightOf(element: HTMLElement): number {
  return element.offsetHeight;
}

/**
 * Gives the parent of an element the height of the element in the run-time variable `name`, as the list keeps room at
 * its end for the bar over its foot ({@link useSizesOn}).
 */
export function useHeightOnParent(
  ref: Readonly<RefObject<HTMLElement | null>>,
  name: RunTimeVariable,
): void {
  const sizes = useMemo(() => [{ name, of: heightOf }], [name]);
  useSizesOn(ref, parentOf, sizes);
}
