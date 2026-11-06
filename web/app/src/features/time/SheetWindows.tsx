import { LeaveGuard } from "../../shared/editing/LeaveGuard.tsx";
import { personName } from "../../shared/person.ts";
import { BreaksWindow } from "./BreaksWindow.tsx";
import { DayHistory } from "./DayHistory.tsx";
import { ReasonWindow } from "./ReasonWindow.tsx";
import { zoneOfDay } from "./sheet-model.ts";
import type { DaysState } from "./use-days.ts";

export interface SheetWindowsProps {
  readonly sheet: DaysState;
  /** The other person whose week is shown; none for one's own. */
  readonly person: string | undefined;
}

/**
 * The windows over the week: the reason a change asks for, the breaks and the history of a day, and
 * the question before the page is left while a change is saved.
 */
export function SheetWindows({ sheet, person }: SheetWindowsProps) {
  const { days, page, shown, history, byDate } = sheet;
  return (
    <>
      <LeaveGuard isBusy={days.editing.isBusy} />
      <ReasonWindow
        editing={days.editing}
        region={days.region}
        other={person === undefined || page === undefined ? undefined : personName(page.person)}
      />
      {sheet.breaksOf === undefined ? null : (
        <BreaksWindow
          date={sheet.breaksOf}
          day={byDate[sheet.breaksOf]}
          region={days.region}
          editing={days.editing}
          onClose={() => {
            sheet.setBreaksOf(undefined);
          }}
        />
      )}
      {shown === undefined ? null : (
        <DayHistory
          date={shown}
          history={history}
          zone={zoneOfDay(byDate[shown], days.region)}
          region={days.region}
          onClose={() => {
            sheet.setShown(undefined);
          }}
        />
      )}
    </>
  );
}
