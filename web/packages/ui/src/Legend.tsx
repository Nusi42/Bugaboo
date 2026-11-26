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

/** The Legend, a small list beside the toolbar: what the row color and the left edge mean (UX §4.2). */
export function Legend({ label, children }: LegendProps) {
  return (
    <ul {...partOf(part.legend)} aria-label={label}>
      {children}
    </ul>
  );
}

/** What a line of the key shows before its words: a swatch of the theme file, or a drawing of its own (a line of the relation map). */
export type LegendSwatch = { readonly look: LegendLook } | { readonly swatch: ReactNode };

export type LegendItemProps = LegendSwatch & {
  /** What the swatch means, in words: color is never alone (R5). */
  readonly children: ReactNode;
};

/** One line of the key: a swatch and what it means. */
export function LegendItem(props: LegendItemProps) {
  return (
    <li {...partOf(part.legendItem)}>
      {"swatch" in props ? (
        props.swatch
      ) : (
        <span {...partOf(part.legendSwatch, { variants: [props.look] })} aria-hidden />
      )}
      {props.children}
    </li>
  );
}
