import { t } from "@bugaboo/messages";

import { addDays } from "../../shared/format.ts";
import { detailsText } from "../../shared/list-text.ts";
import { daysText } from "./dates.ts";
import type { BarDays } from "./lines.ts";
import { hatchedStretches, type HatchedStretch } from "./people/hatched-words.ts";
import { personOnBarText, plannedWithText } from "./people/people-words.ts";
import { known } from "../../shared/present.ts";
import type { TimelineRow } from "./timeline-data.ts";

/** What a bar shows besides its issue and days, for its words. */
export interface BarFacts {
  readonly row: TimelineRow;
  /** The days the bar shows now. */
  readonly shown: BarDays;
  readonly stretches: readonly HatchedStretch[];
  /** How many open plan warnings its issue has. */
  readonly warnings: number;
}

/**
 * The name of a bar for screen readers: its issue and days, then what the eye sees on it, the person
 * on it, the mark of a copy (*"Planned with Lea's work times"*), the named hatched days and the sign
 * of its warnings. Colour and hatching are never alone (R5).
 */
export function barWords(base: string, facts: BarFacts): string {
  const { row, shown } = facts;
  const person = personOnBarText(row);
  const more = [
    person === undefined ? undefined : t("timeline.person-on-bar", { name: person }),
    plannedWithText(row),
    ...facts.stretches.map((stretch) =>
      stretch.words === undefined
        ? undefined
        : t("timeline.hatched-on", {
            what: stretch.words,
            days: daysText({
              first: addDays(shown.first, stretch.start),
              last: addDays(shown.first, stretch.start + stretch.length - 1),
            }),
          }),
    ),
    facts.warnings === 0 ? undefined : t("timeline.warnings-of", { count: facts.warnings }),
  ].filter((said) => said !== undefined);
  return more.length === 0 ? base : t("timeline.bar-with", { bar: base, more: detailsText(more) });
}

/**
 * What a bar says beyond its colours: its hatched days, from the first day it shows, and its name for
 * screen readers. A bar whose person the reader may not see has hatched days without a word.
 */
export function barSaid(row: TimelineRow, shown: BarDays, warnings: number) {
  const named = row.bar?.person !== null && row.bar?.person !== undefined;
  const stretches = hatchedStretches(row.bar?.hatched ?? [], shown.first, named);
  const key = known(row.issue.readable_id) ?? "";
  const base = t("timeline.bar", { key, title: row.issue.title ?? "", days: daysText(shown) });
  return { stretches, name: barWords(base, { row, shown, stretches, warnings }) };
}
