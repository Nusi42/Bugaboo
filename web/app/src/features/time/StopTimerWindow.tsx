import { t } from "@bugaboo/messages";

import type { EntryIssue } from "./entry-issue.ts";
import { EntryWindow } from "./EntryWindow.tsx";
import { StopFooter } from "./StopFooter.tsx";
import { StopTimerBody } from "./StopTimerBody.tsx";
import { closeTimeAsk } from "./time-asks.ts";
import type { EntryDefaults } from "./use-entry-defaults.ts";
import { useStopTimer } from "./use-stop-timer.ts";
import type { TimerReading } from "./use-timer.ts";

export interface StopTimerWindowProps {
  readonly reading: TimerReading;
  /** The issue the timer runs on. */
  readonly issue: EntryIssue;
  /** What a new entry on it starts with. */
  readonly defaults: EntryDefaults;
  /** Another issue to go on with after saving: *Switch issue* needed this window to fill in what is missing. */
  readonly switchingTo?: string | undefined;
}

/**
 * *Stop timer* (TIME-01, UX §4.9): a window with the sentence *You worked 42m on …*, the fields of
 * the entry already filled in from the issue, a note and *Billable*; *Throw away* and *Save time*.
 * Under one minute it says that nothing will be saved.
 */
export function StopTimerWindow({ reading, issue, defaults, switchingTo }: StopTimerWindowProps) {
  const stop = useStopTimer(reading, { issue: issue.key, switchingTo, defaults });
  return (
    <EntryWindow
      title={t("time.stop.title", { issue: issue.key })}
      isOpen
      onOpenChange={closeTimeAsk}
      footer={
        <StopFooter
          isPending={stop.isPending}
          onSave={() => {
            void stop.save();
          }}
        />
      }
    >
      <StopTimerBody
        issue={issue}
        defaults={defaults}
        minutes={stop.minutes}
        values={stop.values}
        onChange={stop.setValues}
        problem={stop.problem}
      />
    </EntryWindow>
  );
}
