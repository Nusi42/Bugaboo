import { t } from "@bugaboo/messages";
import { TextField } from "@bugaboo/ui";
import { useState } from "react";

import { dayText } from "../../shared/format.ts";
import { readDay } from "../../shared/read-day.ts";
import { useRegion } from "../../shared/region.ts";

export interface DayBoxProps {
  readonly label: string;
  /** The day as the calendar writes it (`2027-01-14`); empty while the box holds no day. */
  readonly value: string;
  readonly onChange: (day: string) => void;
}

/**
 * A box for a day, written and read in the person's date format (*Region*), where the browser's own
 * date field would use the format of the device. `onChange` gets the calendar day, or an empty text
 * while what is typed is no day.
 */
export function DayBox({ label, value, onChange }: DayBoxProps) {
  const region = useRegion();
  const [typed, setTyped] = useState(() => (value === "" ? "" : dayText(value, region)));
  const read = readDay(typed, region);
  return (
    <TextField
      label={label}
      value={typed}
      onChange={(text) => {
        setTyped(text);
        onChange(readDay(text, region) ?? "");
      }}
      errorMessage={
        typed.trim() === "" || read !== undefined
          ? undefined
          : t("time.entry.not-a-date", { example: dayText("2027-12-31", region) })
      }
    />
  );
}
