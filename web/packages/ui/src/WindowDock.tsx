import { useEffect, useRef } from "react";
import { Button } from "react-aria-components";

import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface WindowDockProps {
  readonly title: string;
  /** What the minimized window keeps: *Draft kept*. */
  readonly note: string;
  readonly onRestore: () => void;
}

/**
 * A minimized window: a bar at the bottom of the page with its title; pressing it opens the window
 * again. The focus moves to the bar when it appears, because the window that had it is gone (UX §8).
 */
export function WindowDock({ title, note, onRestore }: WindowDockProps) {
  const bar = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    bar.current?.focus();
  }, []);
  return (
    <Button ref={bar} {...partOf(part.windowDock)} onPress={onRestore}>
      <span {...partOf(part.windowDockTitle)}>{title}</span>
      <span {...partOf(part.windowDockNote)}>{note}</span>
    </Button>
  );
}
