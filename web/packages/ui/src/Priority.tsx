import { ArrowDown, ArrowUp, ChevronsUp, Minus, type LucideIcon } from "lucide-react";

import { partOf } from "./parts.ts";
import type { PriorityWord } from "./status.ts";
import { part } from "./tokens.ts";

/** The drawing of each priority: a double arrow up, an arrow up, a line, an arrow down. */
const glyphs: Readonly<Record<PriorityWord, LucideIcon>> = {
  critical: ChevronsUp,
  high: ArrowUp,
  normal: Minus,
  low: ArrowDown,
};

export interface PriorityProps {
  readonly priority: PriorityWord;
  /** The word of the priority, in the person's language. */
  readonly label: string;
  /**
   * Whether the word is shown. A list shows it only for *Critical*; elsewhere the word is the
   * name of the icon, so a priority is never color alone (R5).
   */
  readonly showWord?: boolean;
}

/** A priority: its icon, and its word where there is room. */
export function Priority({ priority, label, showWord = true }: PriorityProps) {
  const Glyph = glyphs[priority];
  return (
    <span
      {...partOf(part.priority, { variants: [priority] })}
      {...(showWord ? {} : { role: "img", "aria-label": label })}
    >
      <Glyph {...partOf(part.priorityIcon)} aria-hidden />
      {showWord ? label : null}
    </span>
  );
}
