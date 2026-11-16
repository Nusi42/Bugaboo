import { TextField } from "@bugaboo/ui";

import { useTypedDay } from "./typed-day.ts";

export interface DayBoxProps {
  readonly label: string;
  /** The day as the calendar writes it (`2027-01-14`); empty while the box holds no day. */
  readonly value: string;
  /** Hands on the day typed (`2027-01-14`), or empty for an empty box. */
  readonly onChange: (day: string) => void;
  /**
   * Whether the text is no day (`true`) or a day or empty again: a form sent as a whole holds its
   * *Save* while it is not (`useMistakes`), so a day half typed is never sent as no day.
   */
  readonly onMistake: (wrong: boolean) => void;
}

/**
 * A box for a day, written and read in the person's date format (*Region*), where the browser's own
 * date field would use the format of the device. `onChange` gets the calendar day, or an empty text
 * for an empty box; text that is no day is told to `onMistake` instead and handed on as nothing. Every box for a day is one of these or is typed through the same
 * `useTypedDay`: no box of the browser writes a day (TXT-DATES).
 */
export function DayBox({ label, value, onChange, onMistake }: DayBoxProps) {
  const day = useTypedDay(value);
  return (
    <TextField
      label={label}
      value={day.typed}
      onChange={(text) => {
        day.setTyped(text);
        const read = day.read(text);
        onMistake(read === undefined);
        if (read !== undefined) {
          onChange(read);
        }
      }}
      errorMessage={day.problem}
    />
  );
}
