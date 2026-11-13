import { usePlace } from "@bugaboo/ui";
import { useRef, type KeyboardEvent } from "react";

import { useDropOf } from "../../shared/drop-of.ts";
import { keyMoveOf } from "./bar-keys.ts";
import { dragKinds } from "./drag-kinds.ts";
import type { BarDays, IssueLine } from "./lines.ts";
import { shownWhileMoving, type Move } from "./moves.ts";
import { placeOf } from "./scale.ts";
import { useTimeline } from "./timeline-context.ts";
import { useBarDrag } from "./use-bar-drag.ts";

/**
 * The keys of a bar: *Enter* and *Space* select it (the press of a pointer is the drag's), an arrow
 * moves it (`keyMoveOf`). The presses of a run are joined into one change (`onKeyMove`).
 */
function keysOf(select: () => void, moveBy: (move: Move, days: number) => void) {
  return (event: KeyboardEvent<HTMLButtonElement>): void => {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      select();
      return;
    }
    const asked = keyMoveOf(event);
    if (asked !== undefined) {
      event.preventDefault();
      moveBy(asked.move, asked.days);
    }
  };
}

/**
 * The keys of a bar and what ends a run of them: a run is saved when the keys are let go (after a
 * short pause) or when the bar loses the focus.
 */
function barKeys(
  timeline: ReturnType<typeof useTimeline>,
  line: IssueLine,
  mayMove: boolean,
  select: () => void,
) {
  return {
    onKeyDown: keysOf(select, (move, by) => {
      if (mayMove) {
        timeline.onKeyMove(line.row, move, by);
      } else {
        timeline.onLocked(line.row);
      }
    }),
    onKeyUp: () => {
      timeline.onRunPause(line.issue);
    },
    onBlur: () => {
      timeline.onRunEnd(line.issue);
    },
  };
}

/**
 * What a bar does (PLAN-03): it is selected by a press, moved by a drag or a key when the person may
 * change its dates (or says why not), takes a dependency dropped on it, and stands on its days.
 */
export function useBar(line: IssueLine, days: BarDays) {
  const timeline = useTimeline();
  const { row } = line;
  const ref = useRef<HTMLDivElement>(null);
  const selected = timeline.selected === line.issue;
  const mayMove = row.may_move && timeline.fields !== undefined;
  const select = (): void => {
    timeline.select(selected ? undefined : line.issue);
  };
  const { dragging, handlers } = useBarDrag({
    dayPixels: timeline.dayPixels,
    mayDrag: mayMove,
    onEnd: (move, by) => {
      timeline.onMove(row, move, by);
    },
    onPress: select,
  });
  const shown = dragging === null ? days : shownWhileMoving(days, dragging.move, dragging.days);
  // Whether a dependency may be made is the server's rule (either issue may be changed): it says
  // why when it refuses.
  const drop = useDropOf(dragKinds.dependency.kind, ref, true, (first) => {
    if (first !== line.issue) {
      timeline.onLink(first, line.issue);
    }
  });
  const start = placeOf(timeline.span, shown.first);
  usePlace(ref, {
    "timeline-start": start,
    "timeline-length": placeOf(timeline.span, shown.last) - start + 1,
    "timeline-done": row.bar?.done ?? 0,
  });
  return {
    ref,
    shown,
    selected,
    mayMove,
    dragging,
    handlers,
    drop,
    ...barKeys(timeline, line, mayMove, select),
  };
}
