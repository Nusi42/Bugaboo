import { t } from "@bugaboo/messages";
import { useLayout, partOf, part } from "@bugaboo/ui";

import { weekFirstOf } from "../../shared/clock.ts";
import { QueryGate } from "../shell/index.ts";
import { PhoneSheet } from "./PhoneSheet.tsx";
import type { IssuesProps } from "./sheet-props.ts";
import { SheetGrid } from "./SheetGrid.tsx";
import { useDays, type DaysInput } from "./use-days.ts";
import { WeekBar } from "./WeekBar.tsx";
import { SheetWindows } from "./SheetWindows.tsx";

export interface WeekProps extends DaysInput {
  readonly onWeek: (first: string) => void;
  readonly issues?: IssuesProps | undefined;
}

/**
 * One week of the timesheet around the rows it is given: the bar of the week, the grid on a
 * computer or the days on a phone, the sentence of an empty week, and the windows of the reason
 * and of the history of a day. Every figure the server works out (the hours of a day, the
 * warnings, the comparison) is shown as it comes.
 */
export function Week(props: WeekProps) {
  const { first, onWeek, issues } = props;
  const layout = useLayout();
  const sheet = useDays(props);
  const { days, page } = sheet;
  const withIssues = issues === undefined ? {} : { issues };
  return (
    <div {...partOf(part.sheet)}>
      <WeekBar
        first={first}
        current={weekFirstOf(days.today, days.region.firstDay)}
        region={days.region}
        state={days.editing.state}
        onWeek={onWeek}
      />
      <QueryGate error={sheet.error} data={page}>
        {() => (
          <>
            {layout === "phone" ? (
              <PhoneSheet days={days} {...withIssues} />
            ) : (
              <SheetGrid days={days} {...withIssues} />
            )}
            {Object.keys(sheet.byDate).length === 0 && (issues?.rows.length ?? 0) === 0 ? (
              <p {...partOf(part.muted)}>{t("time.sheet.empty")}</p>
            ) : null}
            <SheetWindows sheet={sheet} person={props.person} />
          </>
        )}
      </QueryGate>
    </div>
  );
}
