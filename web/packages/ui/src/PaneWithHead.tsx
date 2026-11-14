import { useCallback, useContext, useMemo, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";

import { useHeightOn } from "./height-variable.ts";
import { PaneHeadContext } from "./pane-head.ts";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface PaneWithHeadProps {
  /** What stands above the boxes of the pane: the title, the toolbar, notices. */
  readonly head: ReactNode;
  /** The boxes of the pane: each starts under the whole head, and the first two fold it as they scroll. */
  readonly children: ReactNode;
}

/**
 * Puts what a part deeper in the pane draws above its box (the search bar, the options of a list) into the head of the
 * pane, after what the head holds already, so it folds with the head (UX §6.5). Outside a pane it stands where it is.
 */
export function InPaneHead({ children }: { readonly children: ReactNode }) {
  const { slot } = useContext(PaneHeadContext);
  return slot === null ? children : createPortal(children, slot);
}

/**
 * A pane whose head folds to one line while its boxes scroll, and opens again at their top (UX §6.5). The head is
 * laid over the boxes, which keep the room of the whole head at their start: the browser measures the head whole,
 * and the theme folds it by the scroll of the boxes, so it never jumps and nothing under the pointer moves.
 */
export function PaneWithHead({ head, children }: PaneWithHeadProps) {
  const pane = useRef<HTMLDivElement>(null);
  const whole = useRef<HTMLDivElement>(null);
  const [slot, setSlot] = useState<HTMLElement | null>(null);
  const [above, setAbove] = useState(0);
  const holder = useCallback(() => pane.current, []);
  // The whole head keeps its size inside the layers that fold it, so its end is where it ends whole.
  const onHeight = useCallback(() => {
    const [box, content] = [pane.current, whole.current];
    if (box !== null && content !== null) {
      setAbove(content.getBoundingClientRect().bottom - box.getBoundingClientRect().top);
    }
  }, []);
  useHeightOn(whole, holder, "pane-head-height", onHeight);
  const known = useMemo(() => ({ slot, above }), [slot, above]);
  return (
    <PaneHeadContext value={known}>
      <div ref={pane} {...partOf(part.pane)}>
        <div {...partOf(part.paneHead)}>
          <div {...partOf(part.paneHeadFold)}>
            <div ref={whole} {...partOf(part.paneHeadContent)}>
              {head}
              <div ref={setSlot} {...partOf(part.paneHeadSlot)} />
            </div>
          </div>
        </div>
        {children}
      </div>
    </PaneHeadContext>
  );
}
