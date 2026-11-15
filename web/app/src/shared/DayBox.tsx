import { TextField } from "@bugaboo/ui";

import { useTypedDay } from "./typed-day.ts";

export interface DayBoxProps {
  readonly label: string;
  /** The day as the calendar writes it (`2027-01-14`); empty while the box holds no day. */
  readonly value: string;
  readonly onChange: (day: string) => void;
}

/**
 * A box for a day, written and read in the person's date format (*Region*), where the browser's own
 * date field would use the format of the device. `onChange` gets the calendar day, or an empty text
 * while what is typed is no day. Every box for a day is one of these or is typed through the same
 * `useTypedDay`: no box of the browser writes a day (TXT-DATES).
 */
export function DayBox({ label, value, onChange }: DayBoxProps) {
  const day = useTypedDay(value);
  return (
    <TextField
      label={label}
      value={day.typed}
      onChange={(text) => {
        day.setTyped(text);
        onChange(day.read(text) ?? "");
      }}
      errorMessage={day.problem}
    />
  );
}
