import { t } from "@bugaboo/messages";
import { pixelsOf, part, partOf } from "@bugaboo/ui";
import type { ReactNode } from "react";
import { GridList, ListLayout, Virtualizer } from "react-aria-components";

import type { SearchSpacing } from "../../shared/search-address.ts";

export interface ListScrollProps {
  readonly spacing: SearchSpacing;
  /** The list is being read again: a screen reader is told it is busy. */
  readonly busy: boolean;
  readonly children: ReactNode;
}

/** The group headers are rows that can be reached and pressed but not ticked. */
const keepsHeadersOutOfTheSelection = { disabledBehavior: "selection" } as const;

/**
 * The rows, drawn only where they are on screen (ADR-0009, amendment M5): the list measures the
 * real rows as they appear, so the height asked for here only makes the first guess of the scroll
 * bar a little off.
 */
export function ListScroll({ spacing, busy, children }: ListScrollProps) {
  const guess = pixelsOf(spacing === "compact" ? "size-row-compact" : "size-row");
  return (
    <Virtualizer layout={ListLayout} layoutOptions={{ estimatedRowSize: guess }}>
      <GridList
        aria-label={t("issues.list")}
        aria-busy={busy}
        {...keepsHeadersOutOfTheSelection}
        {...partOf(part.listScroll)}
      >
        {children}
      </GridList>
    </Virtualizer>
  );
}
