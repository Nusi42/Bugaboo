import { t } from "@bugaboo/messages";
import { TextField, fieldKind } from "@bugaboo/ui";

import { boxWords } from "./box-words.ts";
import { SaveForm } from "./SaveForm.tsx";
import { useSynced } from "./use-synced.ts";
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

/** The day and the time of day of a value as the server sent it. */
function partsOf(value: unknown, withTime: boolean): readonly [string, string] {
  return typeof value === "string"
    ? [value.slice(0, 10), withTime ? value.slice(11, 16) : ""]
    : ["", ""];
}

/** The box of the time of day. */
function TimeBox({
  value,
  onChange,
}: {
  readonly value: string;
  readonly onChange: (time: string) => void;
}) {
  return <TextField label={t("fields.time-of-day")} value={value} onChange={onChange} />;
}

/** The day and the time of day a moment holds, each as typed so far and as saved. */
function useMomentParts(value: unknown, withTime: boolean) {
  const [savedDay, savedTime] = partsOf(value, withTime);
  const [day, setDay] = useSynced(savedDay);
  const [time, setTime] = useSynced(savedTime);
  return { savedDay, savedTime, day, setDay, time, setTime };
}

/** A day, and for a day and time also the time of day. */
export function MomentEditor({
  field,
  value,
  onCommit,
  problem,
  isPending = false,
  live = false,
  inRow = false,
}: ValueEditorProps) {
  const withTime = field.kind === "date-time";
  const { savedDay, savedTime, day, setDay, time, setTime } = useMomentParts(value, withTime);
  const sendLive = (nextDay: string, nextTime: string) => {
    if (live) {
      onCommit(momentOf(withTime, nextDay, nextTime));
    }
  };
  return (
    <SaveForm
      name={field.name}
      isPending={isPending}
      live={live}
      inRow={inRow}
      onSubmit={() => {
        if (!inRow || day !== savedDay || time !== savedTime) {
          onCommit(momentOf(withTime, day, time));
        }
      }}
    >
      <TextField
        {...boxWords(field, problem)}
        kind={fieldKind.text}
        date={CALENDAR}
        value={day}
        onChange={(next) => {
          setDay(next);
          sendLive(next, time);
        }}
      />
      {withTime ? (
        <TimeBox
          value={time}
          onChange={(next) => {
            setTime(next);
            sendLive(day, next);
          }}
        />
      ) : null}
    </SaveForm>
  );
}
