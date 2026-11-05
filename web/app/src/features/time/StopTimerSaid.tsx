import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { renderSaid } from "../../shared/said.ts";
import { WithWording } from "../../shared/with-wording.tsx";
import type { EntryIssue } from "./entry-issue.ts";
import { IdsIn } from "./IdsIn.tsx";

export interface StopTimerSaidProps {
  readonly issue: EntryIssue;
  /** Whole minutes the timer has run; under one the window says nothing will be saved. */
  readonly minutes: number;
}

/**
 * What the window says about the time: *You worked 42 minutes on "…"*, in the words of the shared
 * wording, or that nothing will be saved.
 */
export function StopTimerSaid({ issue, minutes }: StopTimerSaidProps) {
  if (minutes === 0) {
    return (
      <p {...partOf(part.text)}>
        <IdsIn sentence={t("time.stop.nothing", { issue: issue.key })} ids={[issue.key]} />
      </p>
    );
  }
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
