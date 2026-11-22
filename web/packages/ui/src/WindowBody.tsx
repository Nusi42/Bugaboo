import { useEffect, useRef, type ReactNode, type RefObject } from "react";

import { focusFirstField } from "./first-field.ts";
import { partOf } from "./parts.ts";
import { useKeyboardScroll } from "./scroll-reach.ts";
import { Skeleton } from "./Skeleton.tsx";
import { isWaiting, windowPhase, type WindowPhase } from "./window-shape.ts";
import { part } from "./tokens.ts";

/** How many grey lines stand in for the form of a window while it is on its way. */
const LOADING_LINES = 6;

export interface WindowBodyProps {
  /** Names the body, so the focus can find its first field. */
  readonly id: string;
  /** While the window waits grey shapes stand in for its form, which is kept out of sight until it is there. */
  readonly phase: WindowPhase;
  /** What is said while it loads: *Loading…*. */
  readonly loadingLabel: string;
  /** The window, which takes the focus when its form has no box to type into. */
  readonly dialog: Readonly<RefObject<HTMLElement | null>>;
  readonly children: ReactNode;
}

/**
 * The part of a window that scrolls (UX §6.3). While its form is on its way it shows grey shapes and
 * says so (UX §6.1); the form is drawn all the same, so what it reads is asked for, but not seen.
 */
export function WindowBody({ id, phase, loadingLabel, dialog, children }: WindowBodyProps) {
  const loading = isWaiting(phase);
  const body = useRef<HTMLDivElement>(null);
  // A body that scrolls with nothing in it that takes the focus (only words, or a form not there yet) takes the focus
  // itself, so the keyboard scrolls it; once it has the focus it keeps it until the focus leaves.
  const reach = useKeyboardScroll(body);
  // The focus goes where the person types first, as a dialog does (WCAG 2.4.3), not to the buttons
  // of the title bar that stand before the form, once the form is there; a window with no box puts
  // it on the window itself.
  useEffect(() => {
    if (!loading && !focusFirstField(id)) {
      dialog.current?.focus();
    }
  }, [id, loading, dialog]);
  return (
    <>
      {/* Outside the busy part and there from the start, so the words put in after 0.3 s are said, once. */}
      <output {...partOf(part.visuallyHidden)}>
        {phase === windowPhase.loading ? loadingLabel : ""}
      </output>
      <div
        ref={body}
        id={id}
        {...partOf(part.windowBody)}
        aria-busy={loading}
        {...(reach ? { tabIndex: 0 } : {})}
      >
        {loading ? <Skeleton lines={LOADING_LINES} /> : null}
        <div {...partOf(part.windowForm, { states: [loading && "loading"] })}>{children}</div>
      </div>
    </>
  );
}
