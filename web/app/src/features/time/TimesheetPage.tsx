import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";

import { weekFirstOf } from "../../shared/clock.ts";
import { useRegion } from "../../shared/region.ts";
import { useNow } from "../../shared/use-now.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { WordingNotice } from "../../shared/with-wording.tsx";
import { todayOf } from "./sheet-model.ts";
import { OwnSheet } from "./Sheet.tsx";

export interface TimesheetPageProps {
  /** The first date of the week shown, from the address; the week of today without it. */
  readonly week: string | undefined;
  readonly onWeek: (first: string) => void;
}

/**
 * *Timesheet* (UX §4.9, TIME-04, TIME-08): the person's week in one grid, the *Work day* rows above
 * the time on issues, with a footer that compares the two. On a phone it is one day at a time.
 */
export function TimesheetPage({ week, onWeek }: TimesheetPageProps) {
  usePageTitle(t("time.sheet.title"));
  const region = useRegion();
  const now = useNow();
  const first = week ?? weekFirstOf(todayOf(region, now), region.firstDay);
  return (
    <Card heading={t("time.sheet.title")} level={1}>
      <WordingNotice />
      <OwnSheet first={first} onWeek={onWeek} />
    </Card>
  );
}
