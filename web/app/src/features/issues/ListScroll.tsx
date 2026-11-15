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

/** The keys that move the focus a page up or down, or to the last row. */
const PAGE_KEYS: ReadonlySet<string> = new Set(["PageUp", "PageDown", "End"]);

/** What React Aria marks every row of the list with: its key. */
const ROW_KEY = "data-key";

/**
 * React Aria moves the focus of a row a page, or to the last row, and brings it into view, but leaves the key to the
 * browser too, which then scrolls the box a page more, or to its end while React Aria scrolls to the row, and the focused
 * row out of view (M8-F225). The box scrolls once, by React Aria. *Home* keeps the scroll of the browser, which brings
 * the box back to its very start, with the head whole (UX §6.5).
 */
function keepOneScroll(event: KeyboardEvent): void {
  if (
    PAGE_KEYS.has(event.key) &&
    event.target instanceof HTMLElement &&
    event.target.hasAttribute(ROW_KEY)
  ) {
    event.preventDefault();
  }
}

/** Gives the box of the rows `keepOneScroll` while it is drawn. */
function scrollsOnce(rows: HTMLElement | null): (() => void) | undefined {
  if (rows === null) {
    return undefined;
  }
  rows.addEventListener("keydown", keepOneScroll);
  return () => {
    rows.removeEventListener("keydown", keepOneScroll);
  };
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
        ref={scrollsOnce}
        {...keepsHeadersOutOfTheSelection}
        {...partOf(part.listScroll)}
      >
        {children}
      </GridList>
    </Virtualizer>
  );
}
