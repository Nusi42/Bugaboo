import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, Checkbox, Length, SaveStatus, partOf, part } from "@bugaboo/ui";

import { weekdayName } from "../../shared/clock.ts";
import { minutesText } from "../../shared/own-value-text.ts";
import { useRegion } from "../../shared/region.ts";
import { saveLabels } from "../../shared/save-labels.ts";
import { LengthBox } from "../fields/index.ts";
import { useUsualDays } from "./use-usual-days.ts";
import type { UsualDay, WorkTimes, WorkTimesRead, Whose } from "./work-times.ts";
import { usualSummary, weekOf } from "./work-times-words.ts";

export interface UsualDaysProps {
  readonly whose: Whose;
  readonly read: WorkTimesRead;
  readonly workTimes: Pick<WorkTimes, "usual" | "usual_version" | "may_change">;
}

/** The seven days with one of them changed. */
function withDay(usual: readonly UsualDay[], at: number, day: UsualDay): readonly UsualDay[] {
  return usual.map((each, index) => (index === at ? day : each));
}

/**
 * The work hours of one day: a box on a ticked day for who may change them; as text otherwise, muted
 * like a box that cannot be used on a day that is not ticked, so nothing that cannot be changed takes
 * the focus.
 */
function DayHours({
  name,
  day,
  mayChange,
  onMinutes,
}: {
  readonly name: string;
  readonly day: UsualDay;
  readonly mayChange: boolean;
  readonly onMinutes: (minutes: number) => void;
}) {
  if (mayChange && day.works) {
    return (
      <LengthBox
        name={t("work-times.day-hours", { day: name })}
        minutes={day.minutes}
        onCommit={(minutes) => {
          onMinutes(minutes ?? 0);
        }}
      />
    );
  }
  const length = <Length text={minutesText(day.minutes)} />;
  return day.works ? length : <span {...partOf(part.hint)}>{length}</span>;
}

/**
 * *Usual work days* (PLAN-11, PLAN-15): the seven days, each with its tick and its work hours, and
 * the line above that sums them up. A tick and the hours save in place, with *Undo* (R7). The rules
 * of what is a work day are the server's.
 *
 * The days stand in the order the *Region* of the reader starts the week (TXT-DATES: dates follow the
 * person who reads them), also on the page of somebody else, whose days the server sends Monday first.
 */
export function UsualDays({ whose, read, workTimes }: UsualDaysProps) {
  const region = useRegion();
  const week = weekOf(region.firstDay);
  const edits = useUsualDays(whose, read);
  const usual = workTimes.usual;
  return (
    <Card heading={t("work-times.usual")} intro={usualSummary(week, usual)}>
      <SaveStatus state={edits.saveState} labels={saveLabels()} />
      {edits.problem === null ? null : <Alert tone={alertTone.danger}>{edits.problem}</Alert>}
      <ul {...partOf(part.list, { variants: ["snug"] })} aria-label={t("work-times.usual")}>
        {week.map(({ at, date }) => {
          const day = usual[at];
          if (day === undefined) {
            return null;
          }
          const name = weekdayName(date, "long");
          const change = (next: UsualDay): void => {
            edits.change({
              days: withDay(usual, at, next),
              version: workTimes.usual_version,
              before: usual,
            });
          };
          return (
            <li key={at} {...partOf(part.item, { variants: ["row", "day"] })}>
              <Checkbox
                label={name}
                isSelected={day.works}
                isDisabled={!workTimes.may_change}
                onChange={(works) => {
                  change({ ...day, works });
                }}
              />
              <DayHours
                name={name}
                day={day}
                mayChange={workTimes.may_change}
                onMinutes={(minutes) => {
                  change({ ...day, minutes });
                }}
              />
            </li>
          );
        })}
      </ul>
    </Card>
  );
}
