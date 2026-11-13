import { useEffect, type ReactNode, type RefObject } from "react";

import { focusFirstField } from "./first-field.ts";

import { partOf } from "./parts.ts";
import { Skeleton } from "./Skeleton.tsx";
import { part } from "./tokens.ts";

/** How many grey lines stand in for the form of a window while it is on its way. */
const LOADING_LINES = 6;

export interface WindowBodyProps {
  /** Names the body, so the focus can find its first field. */
  readonly id: string;
  /** The form is on its way: grey shapes stand in for it, and it is kept out of sight until it is there. */
  readonly loading: boolean;
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
export function WindowBody({ id, loading, loadingLabel, dialog, children }: WindowBodyProps) {
  // The focus goes where the person types first, as a dialog does (WCAG 2.4.3), not to the buttons
  // of the title bar that stand before the form, once the form is there; a window with no box puts
  // it on the window itself.
  useEffect(() => {
    if (!loading && !focusFirstField(id)) {
      dialog.current?.focus();
    }
  }, [id, loading, dialog]);
  return (
    <div id={id} {...partOf(part.windowBody)} aria-busy={loading}>
      {loading ? (
        <>
          <output {...partOf(part.visuallyHidden)}>{loadingLabel}</output>
          <Skeleton lines={LOADING_LINES} />
        </>
      ) : null}
      <div {...partOf(part.windowForm, { states: [loading && "loading"] })}>{children}</div>
    </div>
  );
}
