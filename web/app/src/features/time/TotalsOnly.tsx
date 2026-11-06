import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, partOf, part } from "@bugaboo/ui";

import { formatCalendarDay } from "../../shared/format.ts";
import { useRegion } from "../../shared/region.ts";
import { renderSaid } from "../../shared/said.ts";
import { WithWording } from "../../shared/with-wording.tsx";
import { useThisWeek } from "./use-week.ts";
import { WeekBar } from "./WeekBar.tsx";

export interface TotalsOnlyProps {
  readonly page: Schema<"WorkDaysBody">;
  /** The first date of the week. */
  readonly first: string;
  /** The name of the person. */
  readonly name: string;
  readonly onWeek: (first: string) => void;
}

/**
 * While *Totals only* is on, the week total of the person and nothing else: no days, no starts, ends
 * or breaks, and no changes (ADR-0035, decision 8). The week is still picked as everywhere.
 */
export function TotalsOnly({ page, first, name, onWeek }: TotalsOnlyProps) {
  const region = useRegion();
  const thisWeek = useThisWeek();
  return (
    <WithWording>
      {(wording) => (
        <>
          <WeekBar first={first} current={thisWeek} region={region} onWeek={onWeek} />
          <Alert tone={alertTone.info}>
            {t("time.others.week-of-person", {
              person: name,
              date: formatCalendarDay(first, region),
            })}
            <br />
            {t("time.others.week-worked", {
              worked: renderSaid(wording.lengthSentence(page.weeks[0]?.worked ?? 0)),
            })}
            <br />
            <span {...partOf(part.muted)}>{t("time.others.totals-only-note")}</span>
          </Alert>
        </>
      )}
    </WithWording>
  );
}
