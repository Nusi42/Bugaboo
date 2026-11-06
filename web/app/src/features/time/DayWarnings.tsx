import { t } from "@bugaboo/messages";
import { Icon, iconName, partOf, part } from "@bugaboo/ui";

import { weekdayName } from "../../shared/clock.ts";
import { cellKey, type WarnedDay } from "./sheet-model.ts";

export interface DayWarningsProps {
  /** The sentences to show, each with the date of its day. */
  readonly warned: readonly WarnedDay[];
  /** Whether each sentence names its day: not where one day alone is shown (a phone). */
  readonly named?: boolean;
}

/**
 * The sentences of the working-time rules a day broke, each under the work-day rows with the name
 * of its day, in the warning color and in the words of the server (ADR-0035, decision 6). A warning
 * never blocks anything and is never a box of its own.
 */
export function DayWarnings({ warned, named = true }: DayWarningsProps) {
  if (warned.length === 0) {
    return null;
  }
  return (
    <ul {...partOf(part.sheetWarnings)}>
      {warned.map(({ date, sentence }) => (
        <li key={cellKey(date, sentence)} {...partOf(part.sheetWarning)}>
          <Icon name={iconName.danger} />
          <span>
            {named
              ? t("time.sheet.warning-of-day", { day: weekdayName(date, "long"), sentence })
              : sentence}
          </span>
        </li>
      ))}
    </ul>
  );
}
