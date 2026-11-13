import { useRef } from "react";

import type { MenuAction } from "./ActionMenu.tsx";
import { Button } from "./Button.tsx";
import { partOf } from "./parts.ts";
import { StatusIcon } from "./StatusIcon.tsx";
import type { SwipeSide } from "./swipe.ts";
import { part } from "./tokens.ts";
import { useCloseOutside } from "./use-close-outside.ts";

/**
 * The actions of the open side, under the row's words, sliding in from its edge. Pressing one
 * closes the side, gives the focus to the row's *More* and does the action.
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
  useCloseOutside(strip, close);
  return (
    <div ref={strip} {...partOf(part.swipeActions, { variants: [side] })}>
      {actions.map((action) => (
        <Button
          key={action.id}
          onPress={() => {
            close();
            more.current?.querySelector("button")?.focus();
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
