import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { renderSaid } from "../../shared/said.ts";
import { WithWording } from "../../shared/with-wording.tsx";
import type { EntryIssue } from "./entry-issue.ts";

export interface StopTimerSaidProps {
  readonly issue: EntryIssue;
  /** Whole minutes the timer has run: at least one, or the window is not shown. */
  readonly minutes: number;
}

/**
 * What the window says about the time: *You worked 42 minutes on "…"*, in the words of the shared
 * wording. A timer under a minute has no window (`StopUnderMinute`).
 */
export function StopTimerSaid({ issue, minutes }: StopTimerSaidProps) {
  return (
    <p {...partOf(part.text)}>
      <WithWording>
        {(wording) =>
          t("time.stop.worked", {
            length: renderSaid(wording.lengthSentence(minutes)),
            title: issue.title,
          })
        }
      </WithWording>
    </p>
  );
}
