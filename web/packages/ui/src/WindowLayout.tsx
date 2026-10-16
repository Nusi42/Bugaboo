import type { ReactNode } from "react";

import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface WindowLayoutProps {
  /** The fields that stand at the side once the window is wide enough (880). */
  readonly fields: ReactNode;
  /** The text: the title and the description. */
  readonly children: ReactNode;
}

/**
 * The form of a window follows the window's own width, not the screen's (UX §6.3, R10): one column
 * below 880, the fields at the side from 880, the text centered with a readable width from 1200.
 * The theme file does it with container queries.
 */
export function WindowLayout({ fields, children }: WindowLayoutProps) {
  return (
    <div {...partOf(part.windowLayout)}>
      <div {...partOf(part.windowLayoutText)}>{children}</div>
      <div {...partOf(part.windowLayoutFields)}>{fields}</div>
    </div>
  );
}
