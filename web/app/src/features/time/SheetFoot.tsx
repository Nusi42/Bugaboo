import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { gridLength, idleLength, type Figures } from "./sheet-model.ts";
import { scope } from "./html-words.ts";
import { NAME_COLUMNS, type DaysProps, type IssuesProps } from "./sheet-props.ts";

/**
 * The footer (TIME-04): *Booked on issues* and *Working hours not booked on issues* for each day
 * and for the week, as the server compares them. A day with more booked than worked shows a
 * negative figure, which is the plain truth and not a score.
 */
export function SheetFoot(
  props: Pick<DaysProps, "wording" | "days"> & Pick<IssuesProps, "booked" | "week">,
) {
  const { wording, days, booked, week } = props;
  const shown = (minutes: number | undefined, day: Readonly<Figures> | undefined): string =>
    idleLength(wording, minutes, day);
  const lines = [
    {
      name: "time.sheet.booked",
      of: (date: string) => shown(booked[date]?.booked, booked[date]),
      total: gridLength(wording, week?.booked),
    },
    {
      name: "time.sheet.not-booked",
      of: (date: string) => shown(booked[date]?.notBooked, booked[date]),
      total: gridLength(wording, week?.notBooked),
    },
  ] as const;
  return (
    <tfoot>
      {lines.map(({ name, of, total }) => (
        <tr key={name}>
          <th scope={scope.row} colSpan={NAME_COLUMNS} {...partOf(part.sheetName)}>
            {t(name)}
          </th>
          {days.map((date) => (
            <td key={date} {...partOf(part.sheetCell, { variants: ["figure"] })}>
              <span {...partOf(part.sheetFigure)}>{of(date)}</span>
            </td>
          ))}
          <td {...partOf(part.sheetCell, { variants: ["figure"] })}>
            <span {...partOf(part.sheetFigure)}>{total}</span>
          </td>
        </tr>
      ))}
    </tfoot>
  );
}
