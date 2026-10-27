import { useRef, type KeyboardEvent, type PointerEvent } from "react";

import { pixelsOf } from "./list-template.ts";

/** How a person changes the width of a column. */
export interface ColumnResize {
  /** What the handle is called for screen readers: *Width of Title*. */
  readonly label: string;
  /** The width while the edge is dragged. */
  readonly onResize: (width: number) => void;
  /** The width the drag ended with, or the key set. */
  readonly onResized: (width: number) => void;
}

/** What the handle on the edge of a head cell is given. */
export interface HandleEvents {
  readonly onPointerDown: (event: PointerEvent<HTMLButtonElement>) => void;
  readonly onPointerMove: (event: PointerEvent<HTMLButtonElement>) => void;
  readonly onPointerUp: (event: PointerEvent<HTMLButtonElement>) => void;
  readonly onPointerCancel: (event: PointerEvent<HTMLButtonElement>) => void;
  readonly onKeyDown: (event: KeyboardEvent<HTMLButtonElement>) => void;
}

const keySteps: Readonly<Record<string, number>> = { ArrowRight: 1, ArrowLeft: -1 };

/** A width, no narrower than the least the theme file gives. */
function leastWidth(width: number): number {
  return Math.max(pixelsOf("size-col-least"), Math.round(width));
}

/**
 * Dragging the edge of a head cell with the pointer, or moving it with the arrow keys, sets the
 * width of the column. A column is never narrower than the least the theme file gives.
 */
export function useColumnResize(
  cell: { readonly current: HTMLElement | null },
  resize: ColumnResize | undefined,
): HandleEvents {
  const start = useRef<{ readonly x: number; readonly width: number } | null>(null);
  const widthNow = (): number => cell.current?.getBoundingClientRect().width ?? 0;
  const dragged = (x: number): number =>
    leastWidth((start.current?.width ?? 0) + x - (start.current?.x ?? 0));
  return {
    onPointerDown: (event) => {
      event.currentTarget.setPointerCapture(event.pointerId);
      start.current = { x: event.clientX, width: widthNow() };
    },
    onPointerMove: (event) => {
      if (start.current !== null) {
        resize?.onResize(dragged(event.clientX));
      }
    },
    onPointerUp: (event) => {
      if (start.current !== null) {
        resize?.onResized(dragged(event.clientX));
        start.current = null;
      }
    },
    onPointerCancel: () => {
      start.current = null;
    },
    onKeyDown: (event) => {
      const way = keySteps[event.key];
      if (way !== undefined) {
        event.preventDefault();
        resize?.onResized(leastWidth(widthNow() + way * pixelsOf("size-col-step")));
      }
    },
  };
}
