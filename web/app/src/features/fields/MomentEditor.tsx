import { t } from "@bugaboo/messages";
import { TextField, fieldKind } from "@bugaboo/ui";
import { useState } from "react";

import { boxWords } from "./box-words.ts";
import { SaveForm } from "./SaveForm.tsx";
import type { Sent, ValueEditorProps } from "./ValueEditor.tsx";

/** The widest span of days a day box takes: the whole calendar. */
const CALENDAR = { min: "0001-01-01", max: "9999-12-31" };

/** What the boxes of a day (and a time of day) send: nothing for no day. */
function momentOf(withTime: boolean, day: string, time: string): Sent {
  if (day === "") {
    return null;
  }
  return withTime ? `${day}T${time === "" ? "00:00" : time}:00Z` : day;
}

/** A day, and for a day and time also the time of day. */
export function MomentEditor({
  field,
  value,
  onCommit,
  problem,
  isPending = false,
  live = false,
}: ValueEditorProps) {
  const withTime = field.kind === "date-time";
  const [day, setDay] = useState(typeof value === "string" ? value.slice(0, 10) : "");
  const [time, setTime] = useState(
    typeof value === "string" && withTime ? value.slice(11, 16) : "",
  );
  const sent = (at: string, clock: string): Sent => momentOf(withTime, at, clock);
  return (
    <SaveForm
      isPending={isPending}
      live={live}
      onSubmit={() => {
        onCommit(sent(day, time));
      }}
    >
      <TextField
        {...boxWords(field, problem)}
        kind={fieldKind.text}
        date={CALENDAR}
        value={day}
        onChange={(next) => {
          setDay(next);
          if (live) {
            onCommit(sent(next, time));
          }
        }}
      />
      {withTime ? (
        <TextField
          label={t("fields.time-of-day")}
          value={time}
          onChange={(next) => {
            setTime(next);
            if (live) {
              onCommit(sent(day, next));
            }
          }}
        />
      ) : null}
    </SaveForm>
  );
}
