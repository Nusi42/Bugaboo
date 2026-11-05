import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, partOf, part } from "@bugaboo/ui";

import type { TimePlace } from "../shell/index.ts";
import { useTimeNotice } from "./time-notice.ts";
import { TimerChip } from "./TimerChip.tsx";
import { TimerNotice } from "./TimerNotice.tsx";
import { useTimer } from "./use-timer.ts";

export interface TimePartProps {
  /** Where it stands: in the sidebar (the default), at the top of *Home*, or in the bar of a phone. */
  readonly place?: TimePlace;
  /** The sidebar is narrowed to icons. */
  readonly narrow?: boolean;
}

/** The warning of a timer that runs longer than a work day, on the timer itself. */
function LongTimer({ running }: { readonly running: Schema<"RunningItem"> }) {
  return running.runs_long ? (
    <Alert tone={alertTone.warning}>{t("time.timer.runs-long")}</Alert>
  ) : null;
}

/**
 * The timer and, in the next package of the screen, the work day: one part for the bottom of the
 * sidebar, the top of *Home* and the bar of a phone (TIME-01, TIME-08, UX §3.3, §4.9). It shows
 * nothing while there is no timer and nothing to tell.
 */
export function TimePart({ place = "sidebar", narrow = false }: TimePartProps) {
  const reading = useTimer().data;
  const notice = useTimeNotice();
  const running = reading?.timer.running ?? undefined;
  // The start page shows the work day; the timer is in the bar of a phone and in the sidebar.
  if (place === "home" || (running === undefined && notice === null)) {
    return null;
  }
  return (
    <section
      aria-label={t("time.timer.region")}
      {...partOf(part.timePart, { variants: [place === "bar" && "bar"] })}
    >
      {running === undefined || reading === undefined ? null : (
        <>
          <TimerChip running={running} serverOffsetMs={reading.serverOffsetMs} narrow={narrow} />
          {narrow ? null : <LongTimer running={running} />}
        </>
      )}
      {narrow ? null : <TimerNotice />}
    </section>
  );
}
