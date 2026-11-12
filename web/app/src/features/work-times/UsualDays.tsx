import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, Checkbox, SaveStatus, partOf, part } from "@bugaboo/ui";

import { weekdayName } from "../../shared/clock.ts";
import { useRegion } from "../../shared/region.ts";
import { saveLabels } from "../../shared/save-labels.ts";
import { LengthBox } from "../fields/index.ts";
import { useUsualDays } from "./use-usual-days.ts";
import type { UsualDay, WorkTimesRead, Whose } from "./work-times.ts";
import { usualSummary, weekOf } from "./work-times-words.ts";

export interface UsualDaysProps {
  readonly whose: Whose;
  readonly read: WorkTimesRead;
  readonly usual: readonly UsualDay[];
  /** Whether the reader may change them. */
  readonly mayChange: boolean;
}

/** The seven days with one of them changed. */
function withDay(usual: readonly UsualDay[], at: number, day: UsualDay): readonly UsualDay[] {
  return usual.map((each, index) => (index === at ? day : each));
}

/**
 * *Usual work days* (PLAN-11, PLAN-15): the seven days in the order the person's *Region* starts the
 * week, each with its tick and its work hours, and the line above that sums them up. A tick and the
 * hours save in place, with *Undo* (R7). The rules of what is a work day are the server's.
 */
export function UsualDays({ whose, read, usual, mayChange }: UsualDaysProps) {
  const region = useRegion();
  const week = weekOf(region.firstDay);
  const edits = useUsualDays(whose, read);
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
            edits.change({ days: withDay(usual, at, next), before: usual });
          };
          return (
            <li key={at} {...partOf(part.item, { variants: ["row", "day"] })}>
              <Checkbox
                label={name}
                isSelected={day.works}
                isDisabled={!mayChange}
                onChange={(works) => {
                  change({ ...day, works });
                }}
              />
              {mayChange ? (
                <LengthBox
                  name={t("work-times.day-hours", { day: name })}
                  minutes={day.minutes}
                  onCommit={(minutes) => {
                    change({ ...day, minutes: minutes ?? 0 });
                  }}
                />
              ) : null}
            </li>
          );
        })}
      </ul>
    </Card>
  );
}
