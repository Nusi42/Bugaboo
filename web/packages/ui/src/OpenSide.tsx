import { useLayoutEffect, useRef, useState } from "react";

import type { MenuAction } from "./ActionMenu.tsx";
import { Button } from "./Button.tsx";
import { partOf } from "./parts.ts";
import { StatusIcon } from "./StatusIcon.tsx";
import type { SwipeSide } from "./swipe.ts";
import { part } from "./tokens.ts";
import { useCloseOutside } from "./use-close-outside.ts";

/**
 * The actions of the open side, under the row's words, sliding in from its edge. Pressing one, or *Escape*, closes
 * the side and gives the focus to the row's *More*; an action is done then.
 */
export function OpenSide({
  side,
  actions,
  close,
  more,
}: {
  readonly side: Exclude<SwipeSide, "none">;
  readonly actions: readonly MenuAction[];
  readonly close: () => void;
  readonly more: { readonly current: HTMLElement | null };
}) {
  const strip = useRef<HTMLDivElement>(null);
  const toMore = (): void => {
    close();
    more.current?.querySelector("button")?.focus();
  };
  useCloseOutside(strip, close, toMore);
  // The side slides in from the edge it belongs to as the page is written: before the first picture, the strip reads
  // the direction the browser works out for it.
  const [rightToLeft, setRightToLeft] = useState(false);
  useLayoutEffect(() => {
    const element = strip.current;
    setRightToLeft(element !== null && getComputedStyle(element).direction === "rtl");
  }, []);
  return (
    <div
      ref={strip}
      {...partOf(part.swipeActions, { variants: [side], states: [rightToLeft && "rtl"] })}
    >
      {actions.map((action) => (
        <Button
          key={action.id}
          onPress={() => {
            toMore();
            action.onAction();
          }}
        >
          {action.group === undefined ? null : <StatusIcon group={action.group} />}
          {action.label}
        </Button>
      ))}
    </div>
  );
}
