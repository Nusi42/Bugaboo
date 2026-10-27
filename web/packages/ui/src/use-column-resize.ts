import { useEffect, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";

import { doNothing } from "./do-nothing.ts";
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
  readonly onPointerDown: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerMove: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerUp: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerCancel: (event: PointerEvent<HTMLElement>) => void;
  readonly onKeyDown: (event: KeyboardEvent<HTMLElement>) => void;
}

/** What a screen reader is told of the handle: the width now and the least and the most it can have. */
export interface HandleValue {
  readonly now: number;
  readonly least: number;
  readonly most: number;
}

const keySteps: Readonly<Record<string, number>> = { ArrowRight: 1, ArrowLeft: -1 };

/** A width, no narrower than the least the theme file gives and no wider than the most. */
function clampedWidth(width: number): number {
  return Math.min(
    pixelsOf("size-col-most"),
    Math.max(pixelsOf("size-col-least"), Math.round(width)),
  );
}

/** The width of an element as it is drawn, kept up to date while it changes. */
function useCellWidth(cell: { readonly current: HTMLElement | null }): number {
  const [width, setWidth] = useState(0);
  useEffect(() => {
    const element = cell.current;
    if (element === null || typeof ResizeObserver === "undefined") {
      return doNothing;
    }
    const observer = new ResizeObserver(() => {
      setWidth(Math.round(element.getBoundingClientRect().width));
    });
    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  }, [cell]);
  return width;
}

/**
 * Dragging the edge of a head cell with the pointer, or moving it with the arrow keys, sets the
 * width of the column. A column is never narrower than the least the theme file gives. The width
 * now is read from the cell as it is drawn, for the value the handle tells a screen reader.
 */
export function useColumnResize(
  cell: { readonly current: HTMLElement | null },
  resize: ColumnResize | undefined,
): { readonly events: HandleEvents; readonly value: HandleValue } {
  const start = useRef<{ readonly x: number; readonly width: number } | null>(null);
  const now = useCellWidth(cell);
  const widthNow = (): number => cell.current?.getBoundingClientRect().width ?? 0;
  const dragged = (x: number): number =>
    clampedWidth((start.current?.width ?? 0) + x - (start.current?.x ?? 0));
  return {
    value: { now, least: pixelsOf("size-col-least"), most: pixelsOf("size-col-most") },
    events: {
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
          resize?.onResized(clampedWidth(widthNow() + way * pixelsOf("size-col-step")));
        }
      },
    },
  };
}
