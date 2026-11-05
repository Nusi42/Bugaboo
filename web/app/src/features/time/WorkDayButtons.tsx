import { t, type MessageKey } from "@bugaboo/messages";
import { Button, IconButton, iconName, partOf, part } from "@bugaboo/ui";

import { askTime } from "./time-asks.ts";
import { useTimer } from "./use-timer.ts";
import { useWorkDayActions, type WorkDayAction } from "./use-work-day-actions.ts";
import type { WorkDayNow } from "./work-day-state.ts";

/** What each button is called and does. */
const actionWords: Readonly<Record<WorkDayAction, MessageKey>> = {
  start: "time.work-day.start",
  "break-start": "time.work-day.break-start",
  "break-end": "time.work-day.break-end",
  end: "time.work-day.end",
};

/** The buttons the day allows now: start it, or take a break (or end it) and end the day. */
const allowed: Readonly<Record<WorkDayNow["kind"], readonly WorkDayAction[]>> = {
  none: ["start"],
  working: ["break-start", "end"],
  break: ["break-end", "end"],
  ended: ["start"],
};

export interface WorkDayButtonsProps {
  readonly now: WorkDayNow;
  /** The sidebar is narrowed to icons: one button, the one that starts or ends the day. */
  readonly narrow: boolean;
}

/**
 * *Start work day*, *Start break*, *End break* and *End work day*. *End work day* while a timer runs
 * asks first whether to stop the timer too.
 */
export function WorkDayButtons({ now, narrow }: WorkDayButtonsProps) {
  const act = useWorkDayActions(now);
  const running = useTimer().data?.timer.running ?? undefined;
  const press = (action: WorkDayAction): void => {
    if (action === "end" && running?.readable_id !== undefined && running.readable_id !== null) {
      askTime({ kind: "timer-too", issue: running.readable_id });
    } else {
      void act(action);
    }
  };
  const buttons = allowed[now.kind];
  if (narrow) {
    const only = buttons.includes("end") ? "end" : "start";
    return (
      <IconButton
        icon={only === "end" ? iconName.stop : iconName.play}
        label={t(actionWords[only])}
        onPress={() => {
          press(only);
        }}
      />
    );
  }
  return (
    <div {...partOf(part.workDayButtons)}>
      {buttons.map((action) => (
        <div key={action} {...partOf(part.workDayButton)}>
          <Button
            wide
            onPress={() => {
              press(action);
            }}
          >
            {t(actionWords[action])}
          </Button>
        </div>
      ))}
    </div>
  );
}
