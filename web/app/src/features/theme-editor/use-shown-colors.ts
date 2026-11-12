import type { ColorToken } from "@bugaboo/ui";
import { useState } from "react";

import { colorGroups, colorWords, type ColorGroup } from "./color-words.ts";
import { focusColor } from "./focus-color.ts";
import { changedIn } from "./model.ts";
import type { ThemeValues } from "./themes.ts";

/** The groups open when the editor opens: the first two, and every group whose colors the theme changes. */
export function startsOpen(values: ThemeValues, group: ColorGroup): boolean {
  return (
    group === colorGroups[0] || group === colorGroups[1] || changedIn(values, group).length > 0
  );
}

/** The colors on screen besides those the theme changes, and how to show one. */
export interface ShownColors {
  /** The colors the person chose to change, or was sent to by *Show*. */
  readonly added: readonly ColorToken[];
  /** Puts a color on screen, opens its group and puts the cursor in it. */
  readonly reveal: (token: ColorToken) => void;
}

/** The colors a group shows: those the theme changes, then those the person added that it does not change yet. */
export function shownIn(values: ThemeValues, group: ColorGroup, added: readonly ColorToken[]) {
  return [
    ...changedIn(values, group),
    ...added.filter(
      (token) => colorWords[token].group === group && values.colors?.[token] === undefined,
    ),
  ];
}

/** Which colors the person added to the groups of the editor. */
export function useShownColors(): ShownColors {
  const [added, setAdded] = useState<readonly ColorToken[]>([]);
  return {
    added,
    reveal: (token) => {
      setAdded((before) => (before.includes(token) ? before : [...before, token]));
      // The row is drawn on the next frame: the cursor goes there then.
      window.requestAnimationFrame(() => {
        focusColor(token);
      });
    },
  };
}
