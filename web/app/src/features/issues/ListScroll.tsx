import { t } from "@bugaboo/messages";
import { pixelsOf, part, partOf, usePaneHeadAbove } from "@bugaboo/ui";
import type { ReactNode } from "react";
import { GridList, ListLayout, type ListLayoutOptions, Virtualizer } from "react-aria-components";

import { useCompact } from "./use-compact.ts";

export interface ListScrollProps {
  /** The list is being read again: a screen reader is told it is busy. */
  readonly busy: boolean;
  readonly children: ReactNode;
}

/** The group headers are rows that can be reached and pressed but not ticked. */
const keepsHeadersOutOfTheSelection = { disabledBehavior: "selection" } as const;

/** The options of the rows, and the room above the first row: the box reaches up under the head of its pane (UX §6.5). */
type RoomAboveOptions = Readonly<ListLayoutOptions> & { readonly above?: number };

/** What the list tells its layout when something changed. */
type Changed = Readonly<Parameters<ListLayout<object, RoomAboveOptions>["update"]>[0]>;

/** The layout of the rows, with room above the first one (not around them all, as `padding` would). */
class RoomAboveLayout extends ListLayout<object, RoomAboveOptions> {
  private above = 0;

  override update(context: Changed): void {
    this.above = context.layoutOptions?.above ?? 0;
    super.update(context);
  }

  override shouldInvalidateLayoutOptions(
    next: Readonly<RoomAboveOptions>,
    last: Readonly<RoomAboveOptions>,
  ): boolean {
    return next.above !== last.above || super.shouldInvalidateLayoutOptions(next, last);
  }

  protected override buildCollection(offset = this.above) {
    return super.buildCollection(offset);
  }
}

/**
 * The rows, drawn only where they are on screen (ADR-0009, amendment M5): the list measures the
 * real rows as they appear, so the height asked for here only makes the first guess of the scroll
 * bar a little off.
 */
export function ListScroll({ busy, children }: ListScrollProps) {
  const guess = pixelsOf(useCompact() ? "size-row-compact" : "size-row");
  const above = usePaneHeadAbove();
  return (
    <Virtualizer layout={RoomAboveLayout} layoutOptions={{ estimatedRowSize: guess, above }}>
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
