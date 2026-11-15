import { t } from "@bugaboo/messages";
import { TextField, fieldKind } from "@bugaboo/ui";

import { useTypedDay } from "../../shared/typed-day.ts";
import { boxWords } from "./box-words.ts";
import { SaveForm } from "./SaveForm.tsx";
import { useSynced } from "../../shared/use-synced.ts";
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

/**
 * The day and the time of day a moment holds, each as typed so far and as saved, and what a live
 * form does as each changes: it sends the moment at once, or says that the text of the day is no day,
 * so the form holds back until it is mended (`onMistake`). The day is typed as every box for a day is
 * (`useTypedDay`).
 */
function useMomentParts({ value, field, live, onCommit, onMistake }: MomentEditorProps) {
  const withTime = field.kind === "date-time";
  const [savedDay, savedTime] = partsOf(value, withTime);
  const typed = useTypedDay(savedDay);
  const [time, setTime] = useSynced(savedTime);
  const sendLive = (nextTyped: string, nextTime: string) => {
    if (live !== true) {
      return;
    }
    const day = typed.read(nextTyped);
    if (day === undefined) {
      onMistake?.();
    } else {
      onCommit(momentOf(withTime, day, nextTime));
    }
  };
  return { withTime, savedDay, savedTime, typed, time, setTime, sendLive };
}

/** A day, and for a day and time also the time of day. */
export function MomentEditor(props: MomentEditorProps) {
  const { field, onCommit, problem, isPending = false, live = false, inRow = false } = props;
  const { withTime, savedDay, savedTime, typed, time, setTime, sendLive } = useMomentParts(props);
  const { day } = typed;
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
        {...boxWords(field, problem ?? typed.problem, inRow)}
        kind={fieldKind.text}
        value={typed.typed}
        onChange={(next) => {
          typed.setTyped(next);
          sendLive(next, time);
        }}
      />
      {withTime ? (
        <TimeBox
          hideLabel={inRow}
          value={time}
          onChange={(next) => {
            setTime(next);
            sendLive(typed.typed, next);
          }}
        />
      ) : null}
    </SaveForm>
  );
}
