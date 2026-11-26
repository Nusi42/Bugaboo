import { part, partOf } from "@bugaboo/ui";

import { idOf } from "./calendar-data.ts";
import { CalendarItem } from "./CalendarItem.tsx";
import { Carried } from "./Carried.tsx";
import { MarkLine } from "./MarkLine.tsx";
import { useLinesOf, type Line } from "./use-lines-of.ts";

/** One line of a day. */
function OneLine({ line }: { readonly line: Line }) {
  if ("carried" in line) {
    return <Carried entry={line.carried} />;
  }
  if ("mark" in line) {
    return <MarkLine mark={line.mark} />;
  }
  return (
    <li>
      <CalendarItem entry={line.entry} />
    </li>
  );
}

/** The key of a line among the others of its day. */
function keyOf(line: Line): string {
  if ("carried" in line) {
    return `carried:${idOf(line.carried)}`;
  }
  return "mark" in line ? line.mark.key : idOf(line.entry);
}

/**
 * The lines of a day that fit: the marks and the issues, and at their end the place of a carried issue, which
 * takes the last line when the day is full, so the lines above it never move.
 */
export function DayLines({ day, lines }: { readonly day: string; readonly lines: number }) {
  const all = useLinesOf(day);
  const place = all.find((line) => "carried" in line);
  const real = all.filter((line) => !("carried" in line));
  const shown = place === undefined ? real.slice(0, lines) : [...real.slice(0, lines - 1), place];
  return (
    <ul {...partOf(part.calendarDayItems)}>
      {shown.map((line) => (
        <OneLine key={keyOf(line)} line={line} />
      ))}
    </ul>
  );
}
