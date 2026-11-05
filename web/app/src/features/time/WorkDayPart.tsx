import { t } from "@bugaboo/messages";
import { Length, SentenceWithValue, partOf, part } from "@bugaboo/ui";

import { formatTime } from "../../shared/format.ts";
import { minutesText } from "../../shared/own-value-text.ts";
import { WorkDayButtons } from "./WorkDayButtons.tsx";
import { WorkDayLeftOpen } from "./WorkDayLeftOpen.tsx";
import { useWorkDayNow } from "./use-work-days.ts";
import { breakStartOf, endOf, startOf, type WorkDayNow } from "./work-day-state.ts";

/** The sentence about today's hours with the length in the code font; none while no day has started. */
function line(now: WorkDayNow): { readonly sentence: string; readonly worked: string } | undefined {
  const day = now.today;
  const start = day === undefined ? undefined : startOf(day);
  if (day === undefined || start === undefined) {
    return undefined;
  }
  const worked = minutesText(day.totals.worked);
  const from = formatTime(start);
  const onBreak = breakStartOf(day);
  const end = endOf(day);
  if (now.kind === "break" && onBreak !== undefined) {
    return {
      sentence: t("time.work-day.on-break", { start: formatTime(onBreak), worked }),
      worked,
    };
  }
  if (now.kind === "ended" && end !== undefined) {
    return {
      sentence: t("time.work-day.done", { start: from, end: formatTime(end), worked }),
      worked,
    };
  }
  return { sentence: t("time.work-day.since", { start: from, worked }), worked };
}

/**
 * The work day on screen (TIME-08, UX §4.9): the line of today's hours (*Working since 8:02 · 6h
 * 51m so far*) and the buttons *Start work day*, *Start break*, *End break* and *End work day*, as
 * the server's day allows them. A day left open on an earlier date is told above them.
 */
export function WorkDayPart({ narrow }: { readonly narrow: boolean }) {
  const { now } = useWorkDayNow();
  if (now === undefined) {
    return null;
  }
  const said = narrow ? undefined : line(now);
  return (
    <section aria-label={t("time.work-day.region")} {...partOf(part.workDay)}>
      {now.leftOpen === undefined || narrow ? null : <WorkDayLeftOpen day={now.leftOpen} />}
      {said === undefined ? null : (
        <p {...partOf(part.workDayLine)}>
          <SentenceWithValue sentence={said.sentence} value={said.worked}>
            {(length) => <Length text={length} />}
          </SentenceWithValue>
        </p>
      )}
      <WorkDayButtons now={now} narrow={narrow} />
    </section>
  );
}
