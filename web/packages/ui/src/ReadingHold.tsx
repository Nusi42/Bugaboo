import {
  useCallback,
  useContext,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";

import { useAfterDelay } from "./after-delay.ts";
import { partOf } from "./parts.ts";
import { Skeleton } from "./Skeleton.tsx";
import { HoldContext, HoldWords, useHoldWhileReading, type Report } from "./reading-hold.ts";
import { part } from "./tokens.ts";

/**
 * Holds what it holds hidden, though laid out, until the first reads of the parts in it have answered (each says so
 * with `useHoldWhileReading`), and shows it once, so nothing in it moves after it is seen: parts that come in one after
 * the other would push what stands under them (UX §6.4, the first read). After the wait of the grey shapes it draws its
 * own where the room it keeps starts, taking none (UX §6.1), and while it holds it tells screen readers `busy`. Once
 * shown it stays shown; a read again later does not hide it. A hold is per mount: a new screen (another project, another
 * route) builds a new one. It is no box of its own. What a screen reader hears while it waits comes from `HoldWords`.
 */
export function ReadingHold({ children }: { readonly children: ReactNode }) {
  const busy = useContext(HoldWords);
  const waiting = useRef(new Set<string>());
  const [shown, setShown] = useState(false);
  const report = useCallback<Report>((id, on) => {
    if (on) {
      waiting.current.add(id);
      return;
    }
    waiting.current.delete(id);
    // After the effects of this turn: a part that reads again takes its place in the set at once.
    queueMicrotask(() => {
      if (waiting.current.size === 0) {
        setShown(true);
      }
    });
  }, []);
  // The parts in the hold have said whether they wait (their effects run before this one): with none waiting it shows
  // before the first paint.
  useLayoutEffect(() => {
    if (waiting.current.size === 0) {
      setShown(true);
    }
  }, []);
  const hold = useMemo(() => ({ report, held: !shown }), [report, shown]);
  const late = useAfterDelay(!shown);
  return (
    <HoldContext value={hold}>
      {shown ? null : <output {...partOf(part.visuallyHidden)}>{busy}</output>}
      {shown || !late ? null : (
        <div {...partOf(part.readingHoldShapes)} aria-hidden>
          <Skeleton />
        </div>
      )}
      <div {...partOf(part.readingHold, { states: [!shown && "held"] })}>{children}</div>
    </HoldContext>
  );
}

/**
 * What a part that is still being loaded (a lazy part, as the fallback of its `Suspense`) puts in its place inside a
 * hold: nothing, and the hold waits for it.
 */
export function HoldWhileLoading() {
  useHoldWhileReading(true);
  return null;
}
