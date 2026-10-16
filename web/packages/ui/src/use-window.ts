import { useCallback, useReducer, useState } from "react";

import { useLayout } from "./layout.ts";
import { windowState, type WindowState } from "./props.ts";
import { applyWindowSize, useWindowResize } from "./use-window-resize.ts";
import { firstShape, reduceShape, shapeAction } from "./window-shape.ts";
import type { WindowResize } from "./WindowFrame.tsx";
import type { WindowElement, WindowSize } from "./window-size.ts";

/** What the window is showing: on a phone a full screen, else normal or maximized. */
function stateOf(full: boolean, maximized: boolean): WindowState {
  if (full) {
    return windowState.full;
  }
  return maximized ? windowState.maximized : windowState.normal;
}

export interface UseWindow {
  readonly state: WindowState;
  readonly minimized: boolean;
  readonly minimize: () => void;
  readonly restore: () => void;
  readonly toggleMaximize: () => void;
  readonly resize: WindowResize;
  /** The reference of the element that is the window: it gets its remembered size when it appears. */
  readonly placed: (element: WindowElement | null) => void;
}

/**
 * What a window is doing besides being open: minimized to a bar, maximized, resized. A window
 * that closes opens again as it was first drawn, not minimized and not maximized.
 */
export function useWindow(
  isOpen: boolean,
  size: WindowSize | undefined,
  onResized: ((size: WindowSize) => void) | undefined,
): UseWindow {
  const full = useLayout() === "phone";
  const [shape, act] = useReducer(reduceShape, firstShape);
  const [wasOpen, setWasOpen] = useState(isOpen);
  if (wasOpen !== isOpen) {
    setWasOpen(isOpen);
    act(shapeAction.closed);
  }
  const [element, setElement] = useState<WindowElement | null>(null);
  const resize = useWindowResize(element, onResized);
  const placed = useCallback(
    (modal: WindowElement | null) => {
      setElement(modal);
      if (modal !== null && size !== undefined) {
        applyWindowSize(modal, size);
      }
    },
    [size],
  );
  return {
    state: stateOf(full, shape.maximized),
    minimized: shape.minimized,
    minimize: () => {
      act(shapeAction.minimize);
    },
    restore: () => {
      act(shapeAction.restore);
    },
    toggleMaximize: () => {
      act(shapeAction.toggleMaximize);
    },
    resize,
    placed,
  };
}
