import { t } from "@bugaboo/messages";
import { TextField, fieldKind } from "@bugaboo/ui";

import { dayText } from "../../shared/format.ts";
import { dayProblem, readDay } from "../../shared/read-day.ts";
import { useRegion, type Region } from "../../shared/region.ts";
import { boxWords } from "./box-words.ts";
import { SaveForm } from "./SaveForm.tsx";
import { useSynced } from "./use-synced.ts";
import type { BoxField } from "./fields.ts";
import type { Sent, ValueEditorProps } from "./ValueEditor.tsx";

/** What the editor of a day needs: an own field, or a date of the issue itself (*Due date*). */
type MomentEditorProps = Omit<ValueEditorProps, "field"> & { readonly field: BoxField };

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
  hideLabel,
  onChange,
}: {
  readonly value: string;
  readonly hideLabel: boolean;
  readonly onChange: (time: string) => void;
}) {
  return (
    <TextField
      label={t("fields.time-of-day")}
      hideLabel={hideLabel}
      value={value}
      onChange={onChange}
    />
  );
}

/** The day typed into the box, as the calendar writes it: empty for an empty box, none for text that is no day. */
function typedDay(typed: string, region: Region): string | undefined {
  return typed.trim() === "" ? "" : readDay(typed, region);
}

/**
 * The day and the time of day a moment holds, each as typed so far and as saved, and what a live
 * form does as each changes: it sends the moment at once. The day is written and read in the person's
 * date format (*Region*), as every box for a day is, never by the browser's own date box (TXT-DATES).
 */
function useMomentParts({ value, field, live, onCommit }: MomentEditorProps) {
  const region = useRegion();
  const withTime = field.kind === "date-time";
  const [savedDay, savedTime] = partsOf(value, withTime);
  const [typed, setTyped] = useSynced(savedDay === "" ? "" : dayText(savedDay, region));
  const [time, setTime] = useSynced(savedTime);
  const day = typedDay(typed, region);
  const sendLive = (nextTyped: string, nextTime: string) => {
    if (live === true) {
      onCommit(momentOf(withTime, typedDay(nextTyped, region) ?? "", nextTime));
    }
  };
  const problem = dayProblem(typed, region);
  return { withTime, savedDay, savedTime, typed, setTyped, day, time, setTime, sendLive, problem };
}

/** A day, and for a day and time also the time of day. */
export function MomentEditor(props: MomentEditorProps) {
  const { field, onCommit, problem, isPending = false, live = false, inRow = false } = props;
  const moment = useMomentParts(props);
  const { withTime, savedDay, savedTime, typed, day, time, setTime, sendLive } = moment;
  return (
    <SaveForm
      name={field.name}
      isPending={isPending}
      live={live}
      inRow={inRow}
      onSubmit={() => {
        // Text that is no day is not sent: the box says how to type one, and the saved day stays.
        if (day !== undefined && (!inRow || day !== savedDay || time !== savedTime)) {
          onCommit(momentOf(withTime, day, time));
        }
      }}
    >
      <TextField
        {...boxWords(field, problem ?? moment.problem, inRow)}
        kind={fieldKind.text}
        value={typed}
        onChange={(next) => {
          moment.setTyped(next);
          sendLive(next, time);
        }}
      />
      {withTime ? (
        <TimeBox
          hideLabel={inRow}
          value={time}
          onChange={(next) => {
            setTime(next);
            sendLive(typed, next);
          }}
        />
      ) : null}
    </SaveForm>
  );
}
