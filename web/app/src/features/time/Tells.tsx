import type { Schema } from "@bugaboo/api-client";
import { partOf, part } from "@bugaboo/ui";

import { LongTimer } from "./TimerChip.tsx";
import { TimerNotice } from "./TimerNotice.tsx";

/**
 * What the timer tells: its warning when it runs too long, and the notice of what it did. In a window too
 * low for the sidebar, and beside the narrow rail, they float over the page beside it, so the sidebar never
 * grows (M5-F150).
 */
export function Tells({
  floating,
  narrow,
  running,
}: {
  readonly floating: boolean;
  /** The sidebar is narrowed to icons, so the notice floats beside the rail. */
  readonly narrow: boolean;
  readonly running: Schema<"RunningItem"> | undefined;
}) {
  return (
    <div {...partOf(part.timerNotice, { variants: [floating && "floating", narrow && "narrow"] })}>
      {running === undefined ? null : <LongTimer running={running} />}
      <TimerNotice />
    </div>
  );
}
