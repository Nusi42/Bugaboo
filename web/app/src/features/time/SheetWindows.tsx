import { personName } from "../../shared/person.ts";
import { DayHistory } from "./DayHistory.tsx";
import { ReasonWindow } from "./ReasonWindow.tsx";
import { zoneOfDay } from "./sheet-model.ts";
import type { DaysState } from "./use-days.ts";

export interface SheetWindowsProps {
  readonly sheet: DaysState;
  /** The other person whose week is shown; none for one's own. */
  readonly person: string | undefined;
}

/** The windows over the week: the reason a change asks for, and the history of a day. */
export function SheetWindows({ sheet, person }: SheetWindowsProps) {
  const { days, page, shown, history, byDate } = sheet;
  return (
    <>
      <ReasonWindow
        editing={days.editing}
        region={days.region}
        other={person === undefined || page === undefined ? undefined : personName(page.person)}
      />
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
