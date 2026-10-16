import { useState } from "react";

import { variableOf } from "./variables.ts";
import type { WindowResize } from "./WindowFrame.tsx";
import { resizedBy, sizeAfterKey, type WindowEdge, type WindowSize } from "./window-size.ts";

/** The size of a window on the page right now. */
function sizeOf(element: HTMLElement): WindowSize {
  const { width, height } = element.getBoundingClientRect();
  return { width, height };
}

/**
 * Puts a size on a window. The size is two variables of the theme file (`window-width` and
 * `window-height`); nothing else is set from code (THEME-ONLY). The smallest and the largest size
 * are the theme file's, so what is shown may differ from what is asked for.
 */
export function applyWindowSize(element: HTMLElement, size: WindowSize): void {
  element.style.setProperty(variableOf("window-width"), `${size.width}px`);
  element.style.setProperty(variableOf("window-height"), `${size.height}px`);
}

/** Where a drag began: the edge, the size and the pointer. */
interface Drag {
  readonly edge: WindowEdge;
  readonly from: WindowSize;
  readonly x: number;
  readonly y: number;
}

/**
 * Resizing of a window by the pointer at its edges and by the keys (Ctrl and Alt with an arrow key,
 * UX §6.3, UX-04). `onResized` is called with the size the window ended with, as it is shown, so a
 * size below the smallest is remembered as the smallest.
 */
export function useWindowResize(
  frame: HTMLElement | null,
  onResized: ((size: WindowSize) => void) | undefined,
): WindowResize {
  const [drag, setDrag] = useState<Drag | null>(null);
  const finish = (): void => {
    setDrag(null);
    if (drag !== null && frame !== null) {
      onResized?.(sizeOf(frame));
    }
  };
  return {
    handlers: (edge) => ({
      onPointerDown: (event) => {
        if (frame === null) {
          return;
        }
        event.currentTarget.setPointerCapture(event.pointerId);
        event.preventDefault();
        setDrag({
          edge,
          from: sizeOf(frame),
          x: event.clientX,
          y: event.clientY,
        });
      },
      onPointerMove: (event) => {
        const began = drag;
        if (began !== null && frame !== null) {
          const by = { x: event.clientX - began.x, y: event.clientY - began.y };
          applyWindowSize(frame, resizedBy(began.from, began.edge, by));
        }
      },
      onPointerUp: finish,
      onPointerCancel: finish,
    }),
    onKeyDown: (event) => {
      if (!(event.ctrlKey && event.altKey) || frame === null) {
        return;
      }
      const next = sizeAfterKey(sizeOf(frame), event.key);
      if (next !== undefined) {
        event.preventDefault();
        applyWindowSize(frame, next);
        onResized?.(sizeOf(frame));
      }
    },
  };
}
