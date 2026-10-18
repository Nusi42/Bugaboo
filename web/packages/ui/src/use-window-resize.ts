import { useState } from "react";

import { variableOf } from "./variables.ts";
import type { WindowResize } from "./WindowFrame.tsx";
import {
  keyboardStep,
  menuStep,
  notBelow,
  resizedBy,
  sizeAfterStep,
  stepOfArrow,
  type WindowEdge,
  type WindowStep,
  type WindowElement,
  type WindowSize,
} from "./window-size.ts";

/** The smallest height a person can make a window, from the theme file (`size-window-height-min`). */
function smallestHeight(): number {
  // A test of the logic has no page and so no theme: there is no smallest height then.
  if (typeof document === "undefined") {
    return 0;
  }
  const written = getComputedStyle(document.documentElement).getPropertyValue(
    "--size-window-height-min",
  );
  const found = Number(/[\d.]+/u.exec(written)?.[0]);
  return Number.isNaN(found) ? 0 : found;
}

/** The size of a window on the page right now. */
function sizeOf(element: Pick<WindowElement, "getBoundingClientRect">): WindowSize {
  const { width, height } = element.getBoundingClientRect();
  return { width, height };
}

/**
 * Puts a size on a window. The size is two variables of the theme file (`window-width` and
 * `window-height`); nothing else is set from code (THEME-ONLY). The smallest and the largest size
 * are the theme file's, so what is shown may differ from what is asked for.
 */
export function applyWindowSize(element: Pick<WindowElement, "style">, size: WindowSize): void {
  element.style.setProperty(variableOf("window-width"), `${size.width}px`);
  element.style.setProperty(variableOf("window-height"), `${size.height}px`);
}

/** The part of a keyboard event that resizing reads. */
export interface ResizeKey {
  readonly ctrlKey: boolean;
  readonly altKey: boolean;
  readonly key: string;
  readonly preventDefault: () => void;
}

/** Changes the size of a window by a step of `by` pixels and tells what it ended with. */
function stepBy(
  frame: WindowElement | null,
  step: WindowStep,
  by: number,
  onResized: ((size: WindowSize) => void) | undefined,
): void {
  if (frame === null) {
    return;
  }
  const now = sizeOf(frame);
  applyWindowSize(frame, notBelow(sizeAfterStep(now, step, by), smallestHeight(), now.height));
  onResized?.(sizeOf(frame));
}

/** Ctrl and Alt with an arrow key change the size by a step; any other key is left alone. */
export function resizeByKey(
  frame: WindowElement | null,
  event: ResizeKey,
  onResized: ((size: WindowSize) => void) | undefined,
): void {
  const step = stepOfArrow(event.key);
  if (!(event.ctrlKey && event.altKey) || frame === null || step === undefined) {
    return;
  }
  event.preventDefault();
  stepBy(frame, step, keyboardStep, onResized);
}

/** Where a drag began: the edge, the size and the pointer. */
interface Drag {
  readonly edge: WindowEdge;
  readonly from: WindowSize;
  readonly x: number;
  readonly y: number;
}

/**
 * Resizing of a window by the pointer at its edges, by the keys (Ctrl and Alt with an arrow key,
 * UX §6.3, UX-04) and by the size menu, which needs no dragging (WCAG 2.5.7). `onResized` is called with the size the window ended with, as it is shown, so a
 * size below the smallest is remembered as the smallest.
 */
export function useWindowResize(
  frame: WindowElement | null,
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
          applyWindowSize(
            frame,
            notBelow(resizedBy(began.from, began.edge, by), smallestHeight(), began.from.height),
          );
        }
      },
      onPointerUp: finish,
      onPointerCancel: finish,
    }),
    onKeyDown: (event) => {
      resizeByKey(frame, event, onResized);
    },
    step: (step) => {
      stepBy(frame, step, menuStep, onResized);
    },
  };
}
