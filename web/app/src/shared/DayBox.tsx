import { TextField } from "@bugaboo/ui";
import { useState } from "react";

import { dayText } from "./format.ts";
import { dayProblem, readDay } from "./read-day.ts";
import { useRegion } from "./region.ts";

export interface DayBoxProps {
  readonly label: string;
  /** The day as the calendar writes it (`2027-01-14`); empty while the box holds no day. */
  readonly value: string;
  readonly onChange: (day: string) => void;
}

/**
 * A box for a day, written and read in the person's date format (*Region*), where the browser's own
 * date field would use the format of the device. `onChange` gets the calendar day, or an empty text
 * while what is typed is no day. Every box for a day is one of these or reads and writes its text the
 * same way (`dayText`, `readDay`, `dayProblem`): no box of the browser writes a day (TXT-DATES).
 */
export function DayBox({ label, value, onChange }: DayBoxProps) {
  const region = useRegion();
  const [typed, setTyped] = useState(() => (value === "" ? "" : dayText(value, region)));
  return (
    <TextField
      label={label}
      value={typed}
      onChange={(text) => {
        setTyped(text);
        onChange(readDay(text, region) ?? "");
      }}
      errorMessage={dayProblem(typed, region)}
    />
  );
}
