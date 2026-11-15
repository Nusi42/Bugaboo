import {
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
} from "react";
import { createPortal } from "react-dom";

import { doNothing } from "./do-nothing.ts";
import { useSizesOn, type RunTimeSize } from "./height-variable.ts";
import {
  PaneHeadContext,
  paneLook,
  useOnPaper,
  type PaneHeadSlot,
  type PaneLook,
} from "./pane-head.ts";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface PaneWithHeadProps {
  /**
   * What stands above the boxes of the pane: the title, the toolbar, notices (none where the parts of the page put all of
   * it into the slots of the head). Its first line is what it folds to.
   */
  readonly head: ReactNode;
  /** Whether the head folds over the boxes (a computer) or stands in the page above them. */
  readonly look: PaneLook;
  /** The boxes of the pane: each starts under the whole head, and the first two fold it as they scroll. */
  readonly children: ReactNode;
}

/**
 * Puts what a part deeper in the pane draws above its box (the search bar, the options of a list) into its slot of the
 * head of the pane, so it folds with the head (UX §6.5). Outside a pane it stands where it is; in a pane it waits for
 * its slot, so it is drawn once, in its place.
 */
export function InPaneHead({
  slot,
  children,
}: {
  readonly slot: PaneHeadSlot;
  readonly children: ReactNode;
}) {
  const { slots } = useContext(PaneHeadContext);
  if (slots === undefined) {
    return children;
  }
  const into = slots[slot];
  return into === null ? null : createPortal(children, into);
}

/** The focus that the keyboard shows: a control reached with the keyboard, not one pressed with the pointer. */
const KEYBOARD_FOCUS = ":focus-visible";

/**
 * Whether a control in the element has the focus of the keyboard: while it has, the head is whole, so what has the
 * focus shows (UX §6.5).
 */
function useKeyboardFocusIn(ref: Readonly<RefObject<HTMLElement | null>>): boolean {
  const [held, setHeld] = useState(false);
  useEffect(() => {
    const element = ref.current;
    if (element === null) {
      return doNothing;
    }
    const look = (): void => {
      const active = document.activeElement;
      setHeld(active !== null && element.contains(active) && active.matches(KEYBOARD_FOCUS));
    };
    element.addEventListener("focusin", look);
    element.addEventListener("focusout", look);
    return () => {
      element.removeEventListener("focusin", look);
      element.removeEventListener("focusout", look);
    };
  }, [ref]);
  return held;
}

/** Where the whole head ends below the top of the pane: it keeps its size inside the layers that fold it. */
function endOf(content: HTMLElement, pane: HTMLElement): number {
  return content.getBoundingClientRect().bottom - pane.getBoundingClientRect().top;
}

/**
 * How high the first line of the head is: its first part that the browser draws, also one that a deeper part put into a
 * slot (a slot is no box of its own).
 */
function firstLineOf(content: HTMLElement): number {
  for (const child of content.children) {
    if (!(child instanceof HTMLElement)) {
      continue;
    }
    const line = child.matches(SLOT) ? firstLineOf(child) : child.offsetHeight;
    if (line > 0) {
      return line;
    }
  }
  return 0;
}

/** A slot of the head, in which deeper parts put their lines. */
const SLOT = `[data-part='${part.paneHeadSlot}']`;

/** The two sizes of the head the theme folds it by, measured once for the theme and the code. */
const headSizes: readonly RunTimeSize[] = [
  { name: "pane-head-end", of: endOf },
  { name: "pane-head-line", of: firstLineOf },
];

/**
 * A pane whose head folds to its first line while its boxes scroll, and opens again at their top (UX §6.5). The head is
 * laid over the boxes, which keep the room of the whole head at their start: the browser measures the head whole,
 * and the theme folds it by the scroll of the boxes, so it never jumps and nothing under the pointer moves. A pane that
 * does not fold keeps the same parts in the same places, so nothing is drawn afresh when the layout changes.
 */
export function PaneWithHead({ head, look, children }: PaneWithHeadProps) {
  const pane = useRef<HTMLDivElement>(null);
  const whole = useRef<HTMLDivElement>(null);
  const [title, setTitle] = useState<HTMLElement | null>(null);
  const [search, setSearch] = useState<HTMLElement | null>(null);
  const [view, setView] = useState<HTMLElement | null>(null);
  const [end, setEnd] = useState(0);
  const top = useRef<HTMLDivElement>(null);
  const held = useKeyboardFocusIn(top);
  const onPaper = useOnPaper();
  const holder = useCallback(() => pane.current, []);
  const onSizes = useCallback((sizes: readonly number[]) => {
    setEnd(sizes[0] ?? 0);
  }, []);
  useSizesOn(whole, holder, headSizes, onSizes);
  const above = look === paneLook.folds && !onPaper ? end : 0;
  const known = useMemo(
    () => ({ slots: { title, search, view }, above }),
    [title, search, view, above],
  );
  return (
    <PaneHeadContext value={known}>
      <div ref={pane} {...partOf(part.pane, { variants: [look], states: [held && "whole"] })}>
        <div ref={top} {...partOf(part.paneHead)}>
          <div {...partOf(part.paneHeadFold)}>
            <div ref={whole} {...partOf(part.paneHeadContent)}>
              <div ref={setTitle} {...partOf(part.paneHeadSlot)} />
              {head}
              <div ref={setSearch} {...partOf(part.paneHeadSlot)} />
              <div ref={setView} {...partOf(part.paneHeadSlot)} />
            </div>
          </div>
        </div>
        {children}
      </div>
    </PaneHeadContext>
  );
}
