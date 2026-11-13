import { t } from "@bugaboo/messages";

import { unbroken } from "../../../shared/format.ts";
import { detailsText, listText } from "../../../shared/list-text.ts";
import { percentText } from "../../../shared/number.ts";
import { minutesText } from "../../../shared/own-value-text.ts";
import { personName } from "../../../shared/person.ts";
import type { FirstDay } from "../../../shared/region.ts";
import { AS_ESTIMATED, factorText } from "../../../shared/time-factor-text.ts";
import { usualSummary, weekOf } from "../../work-times/index.ts";
import type { TimelineRow } from "../timeline-data.ts";

/**
 * The words of the people on the timeline (UX §4.7, ADR-0030 and its amendment M8): the person on a
 * bar, a copy of *Group by person*, the effort and the person's time. Every word
 * is made from what the server answered for the reader; nothing here works out a day or a length.
 */

/** The bar of a row, as the server answers it. */
type Bar = NonNullable<TimelineRow["bar"]>;

/** The person on a bar as the bar says it: the name, and "+2" for the others assigned. */
export function personOnBarText(row: TimelineRow): string | undefined {
  const person = row.bar?.person;
  if (person === null || person === undefined) {
    return undefined;
  }
  const others = (row.issue.assigned_to?.length ?? 1) - 1;
  const name = personName(person);
  return others > 0 ? t("timeline.person-more", { name, count: others }) : name;
}

/**
 * The mark of a copy in *Group by person*: "Planned with Lea's work times"; none for the bar itself.
 * The card says it of an issue that has a copy on the page (`copied`).
 */
export function plannedWithText(row: TimelineRow, copied = row.copy): string | undefined {
  const person = row.bar?.person;
  return copied && person !== null && person !== undefined
    ? t("timeline.planned-with", { name: personName(person) })
    : undefined;
}

/** Everybody the issue is assigned to, as the reader may see them: the card lists them all. */
export function assignedText(row: TimelineRow): string | undefined {
  const people = row.issue.assigned_to ?? [];
  return people.length === 0
    ? undefined
    : t("timeline.assigned", { people: listText(people.map((person) => personName(person))) });
}

/**
 * The effort of a bar, as the card says it (UX §4.7): "Estimate 16h · ×1.25 = 3 work days · Lea's own
 * estimate", the time factor always named, ×1 too. The work days are there when the bars are planned
 * by effort; without them the sum ends in the length of the effort, as the window of the time factor
 * says it. Lengths are written as everywhere, in hours and minutes (UX §4.9).
 */
export function effortText(bar: Bar): string | undefined {
  const { effort } = bar;
  if (effort === null) {
    return undefined;
  }
  const estimate = minutesText(effort.estimate);
  const factor = factorText(bar.time_factor?.percent ?? AS_ESTIMATED);
  const sum =
    effort.work_days === null
      ? t("issues.time-factor-sum", { estimate, factor, total: minutesText(effort.effort) })
      : t("timeline.effort-days", { estimate, factor, days: effort.work_days });
  const person = bar.person;
  return bar.time_factor?.own_estimate === true && person !== null
    ? t("issues.time-factor-own", { factor: sum, name: personName(person) })
    : sum;
}

/**
 * The person's time on a bar: "Mon–Fri, 8h a day · 100 % in Mobile app", as far as the reader may
 * know it. The line breaks only between its two parts.
 */
export function workTimeText(bar: Bar, project: string, firstDay: FirstDay): string | undefined {
  const time = bar.work_time;
  return time === null
    ? undefined
    : detailsText([
        unbroken(usualSummary(weekOf(firstDay), time.usual)),
        unbroken(t("timeline.work-share", { share: percentText(time.share), project })),
      ]);
}
