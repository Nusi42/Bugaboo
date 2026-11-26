import { part, partOf } from "@bugaboo/ui";
import type { ReactNode } from "react";

import type { LineLook } from "./line-look.ts";

/** The drawing box of a swatch, and its line across the middle. */
const swatchBox = "0 0 24 12";
const swatchLine = "M0 6H24";
/** The piece of line fills the swatch, however wide the theme makes it. */
const stretched = "none";

/**
 * A short piece of the line of a kind (UX §4.15): the same part as the lines of the picture, so its
 * color and its pattern under forced colors are theirs. It decorates words that say the same.
 */
export function LineSwatch({ look }: { readonly look: LineLook }) {
  return (
    <svg
      {...partOf(part.mapSwatch)}
      viewBox={swatchBox}
      preserveAspectRatio={stretched}
      aria-hidden
    >
      <path {...partOf(part.mapLine, { variants: [look] })} d={swatchLine} />
    </svg>
  );
}

/** One line of the Legend of the lines: the swatch of a kind and what it means. */
export function LineLegendItem({
  look,
  children,
}: {
  readonly look: LineLook;
  readonly children: ReactNode;
}) {
  return (
    <li {...partOf(part.legendItem)}>
      <LineSwatch look={look} />
      {children}
    </li>
  );
}
