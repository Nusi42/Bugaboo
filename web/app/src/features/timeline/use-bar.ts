import { useRef, type KeyboardEvent } from "react";
import { usePlace } from "@bugaboo/ui";

import { useDropOf } from "../../shared/drop-of.ts";
import { keyMoveOf } from "./bar-keys.ts";
import { shownWhileMoving, type Move } from "./dates.ts";
import { dragKinds } from "./drag-kinds.ts";
import type { BarDays, IssueLine } from "./lines.ts";
import { placeOf } from "./scale.ts";
import { useTimeline } from "./timeline-context.ts";
import { useBarDrag } from "./use-bar-drag.ts";

/**
 * The keys of a bar: *Enter* and *Space* select it (the press of a pointer is the drag's), an arrow
 * moves it (`keyMoveOf`).
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
  const moveBy = (move: Move, by: number): void => {
    if (mayMove) {
      timeline.onMove(row, move, by);
    } else {
      timeline.onLocked(row);
    }
  };
  const { dragging, handlers } = useBarDrag(timeline.dayPixels, moveBy, select);
  const shown = dragging === null ? days : shownWhileMoving(days, dragging.move, dragging.days);
  const drop = useDropOf(dragKinds.dependency, ref, mayMove, (first) => {
    if (first !== line.issue) {
      timeline.onLink(first, line.issue);
    }
  });
  const start = placeOf(timeline.span, shown.first);
  usePlace(ref, {
    "timeline-start": start,
    "timeline-length": placeOf(timeline.span, shown.last) - start + 1,
    "timeline-done": `${String(row.bar?.done ?? 0)}%`,
  });
  return {
    ref,
    shown,
    selected,
    mayMove,
    dragging,
    handlers,
    drop,
    onKeyDown: keysOf(select, moveBy),
  };
}
