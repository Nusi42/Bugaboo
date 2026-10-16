import { Button } from "react-aria-components";

import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface WindowDockProps {
  readonly title: string;
  /** What the minimized window keeps: *Draft kept*. */
  readonly note: string;
  readonly onRestore: () => void;
}

/** A minimized window: a bar at the bottom of the page with its title; pressing it opens the window again. */
export function WindowDock({ title, note, onRestore }: WindowDockProps) {
  return (
    <Button {...partOf(part.windowDock)} onPress={onRestore}>
      <span {...partOf(part.windowDockTitle)}>{title}</span>
      <span {...partOf(part.windowDockNote)}>{note}</span>
    </Button>
  );
}
