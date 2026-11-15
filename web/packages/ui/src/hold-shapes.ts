import { useLayoutEffect, type RefObject } from "react";

import type { RunTimeVariable } from "./tokens.ts";
import { variableOf } from "./variables.ts";

/** Gives the grey shapes of a hold, in run-time variables, where its room stands in their box, and its size. */
function place(shapes: HTMLElement, room: Readonly<DOMRect>): void {
  const box = shapes.offsetParent;
  const from = box?.getBoundingClientRect() ?? { left: 0, top: 0 };
  const top = room.top - from.top - (box?.clientTop ?? 0) + (box?.scrollTop ?? 0);
  const left = room.left - from.left - (box?.clientLeft ?? 0) + (box?.scrollLeft ?? 0);
  const values: readonly (readonly [RunTimeVariable, number])[] = [
    ["hold-top", top],
    ["hold-left", left],
    ["hold-width", room.width],
    ["hold-height", room.height],
  ];
  for (const [name, value] of values) {
    shapes.style.setProperty(variableOf(name), String(value));
  }
}

/**
 * Lays the grey shapes of a hold over the room it keeps, before they are painted: the hold is no box of its own, so the
 * shapes are placed by the room of what it holds, as the browser lays it out (all of it, hidden).
 */
export function useShapesOverRoom(
  shapes: Readonly<RefObject<HTMLElement | null>>,
  held: Readonly<RefObject<HTMLElement | null>>,
  drawn: boolean,
): void {
  useLayoutEffect(() => {
    const element = shapes.current;
    const room = held.current;
    if (!drawn || element === null || room === null) {
      return;
    }
    const range = document.createRange();
    range.selectNodeContents(room);
    place(element, range.getBoundingClientRect());
  }, [shapes, held, drawn]);
}
