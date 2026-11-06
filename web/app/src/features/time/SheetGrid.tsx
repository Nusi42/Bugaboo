import { partOf, part } from "@bugaboo/ui";
import { t } from "@bugaboo/messages";

import { DayWarnings } from "./DayWarnings.tsx";
import { IssueRows } from "./IssueRows.tsx";
import { NAME_COLUMNS, type DaysProps, type IssuesProps } from "./sheet-props.ts";
import { SheetFoot } from "./SheetFoot.tsx";
import { SheetHead } from "./SheetHead.tsx";
import { WorkDayRows } from "./WorkDayRows.tsx";

/** The widths of the columns, the same for every week: the issue, its two fields, the days and the total. */
function SheetColumns({ days }: { readonly days: number }) {
  return (
    <colgroup>
      <col {...partOf(part.sheetCol, { variants: ["issue"] })} />
      <col {...partOf(part.sheetCol, { variants: ["field"] })} />
      <col {...partOf(part.sheetCol, { variants: ["cost-center"] })} />
      {Array.from({ length: days }, (_, at) => (
        <col key={at} {...partOf(part.sheetCol, { variants: ["day"] })} />
      ))}
      <col {...partOf(part.sheetCol, { variants: ["total"] })} />
    </colgroup>
  );
}

/**
 * The week on a computer (UX §4.9): the *Work day* rows above the time on issues, one column for
 * each day, one grid so the days stand over each other. Without `issues` it is the work-day rows
 * alone, as *Working hours of others* shows them.
 */
export function SheetGrid({
  days,
  issues,
}: {
  readonly days: DaysProps;
  readonly issues?: IssuesProps;
}) {
  return (
    <div {...partOf(part.sheetScroll)}>
      <table {...partOf(part.sheetTable)} aria-label={t("time.sheet.title")}>
        <SheetColumns days={days.days.length} />
        <SheetHead {...days} />
        <tbody>
          <WorkDayRows {...days} />
          <tr>
            <td colSpan={days.days.length + NAME_COLUMNS + 1}>
              <DayWarnings warned={days.warned} />
            </td>
          </tr>
          {issues === undefined ? null : <IssueRows {...days} {...issues} />}
        </tbody>
        {issues === undefined ? null : (
          <SheetFoot {...days} booked={issues.booked} week={issues.week} />
        )}
      </table>
    </div>
  );
}
