import type { ReactNode } from "react";

import { partOf } from "./parts.ts";
import { part, partWords } from "./tokens.ts";

/** What a swatch stands for: a status group's tint, or the edge of a priority. */
export type LegendLook = (typeof partWords)["legend.swatch"]["variant"][number];

export interface LegendProps {
  /** What the key is for, for screen readers: *What the colors mean*. */
  readonly label: string;
  readonly children: ReactNode;
}

/** A small key beside the toolbar: what the row color and the left edge mean (UX §4.2). */
export function Legend({ label, children }: LegendProps) {
  return (
    <ul {...partOf(part.legend)} aria-label={label}>
      {children}
    </ul>
  );
}

export interface LegendItemProps {
  readonly look: LegendLook;
  /** What the swatch means, in words: color is never alone (R5). */
  readonly children: ReactNode;
}

/** One line of the key: a swatch and what it means. */
export function LegendItem({ look, children }: LegendItemProps) {
  return (
    <li {...partOf(part.legendItem)}>
      <span {...partOf(part.legendSwatch, { variants: [look] })} aria-hidden />
      {children}
    </li>
  );
}
