import { useCallback, useRef, type ReactNode } from "react";

import { useHeightOn } from "./height-variable.ts";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface PaneWithHeadProps {
  /** What stands above the boxes of the pane: the title, the toolbar, notices. */
  readonly head: ReactNode;
  /** The boxes of the pane: each starts under the whole head, and the first two fold it as they scroll. */
  readonly children: ReactNode;
}

/**
 * A pane whose head folds to one line while its boxes scroll, and opens again at their top (UX §6.5). The head is
 * laid over the boxes, which keep the room of the whole head at their start: the browser measures the head whole,
 * and the theme folds it by the scroll of the boxes, so it never jumps and nothing under the pointer moves.
 */
export function PaneWithHead({ head, children }: PaneWithHeadProps) {
  const pane = useRef<HTMLDivElement>(null);
  const whole = useRef<HTMLDivElement>(null);
  const holder = useCallback(() => pane.current, []);
  useHeightOn(whole, holder, "pane-head-height");
  return (
    <div ref={pane} {...partOf(part.pane)}>
      <div {...partOf(part.paneHead)}>
        <div {...partOf(part.paneHeadFold)}>
          <div ref={whole} {...partOf(part.paneHeadContent)}>
            {head}
          </div>
        </div>
      </div>
      {children}
    </div>
  );
}
