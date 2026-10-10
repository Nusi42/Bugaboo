import { t } from "@bugaboo/messages";
import {
  Button,
  buttonLook,
  Checkbox,
  Form,
  SelectField,
  SubmitButton,
  Switch,
  TextField,
  partOf,
  part,
  fieldKind,
} from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

import type { Field, KindWord } from "./fields.ts";
import { ValueView } from "./ValueView.tsx";
import { durationSeconds, durationText, isDecimalText, isWholeNumberText } from "./value-text.ts";

/** The value the editor sends: what the API takes for this field, or `null` to take the value away. */
export type Sent = string | number | boolean | readonly string[] | null;

export interface ValueEditorProps {
  readonly field: Field;
  /** The value now, as the API words it; nothing when there is none. */
  readonly value: unknown;
  /** Sends a new value; `null` takes the value away. */
  readonly onCommit: (value: Sent) => void;
  /** What the server said was wrong with the last value, in words. */
  readonly problem?: string | undefined;
  readonly isPending?: boolean;
}

/** The choices a person can pick: those not retired, and the ones the value has now. */
function pickable(field: Field, current: readonly string[]) {
  return field.choices.filter((choice) => !choice.retired || current.includes(choice.id));
}

/** The words every box of a field carries: its name, its help text and what is wrong with it. */
function boxWords(field: Field, error?: string) {
  return {
    label: field.name,
    ...(field.help === "" ? {} : { description: field.help }),
    ...(error === undefined ? {} : { errorMessage: error }),
  };
}

/** The text a value has in a box: what a person edits. */
function textOf(field: Field, value: unknown): string {
  if (value === undefined || value === null) {
    return "";
  }
  if (field.kind === "duration" && typeof value === "number") {
    return durationText(value);
  }
  return typeof value === "string" || typeof value === "number" ? String(value) : "";
}

/** What the text of a box means for the API, or a sentence that says why it cannot be sent. */
function sentFrom(
  field: Field,
  text: string,
): { readonly value: Sent } | { readonly problem: string } {
  const trimmed = text.trim();
  if (trimmed === "") {
    return { value: null };
  }
  if (field.kind === "whole-number") {
    return isWholeNumberText(trimmed)
      ? { value: Number(trimmed) }
      : { problem: t("fields.not-a-whole-number") };
  }
  if (field.kind === "decimal") {
    return isDecimalText(trimmed) ? { value: trimmed } : { problem: t("fields.not-a-decimal") };
  }
  if (field.kind === "duration") {
    const seconds = durationSeconds(trimmed);
    return seconds === undefined ? { problem: t("fields.not-a-duration") } : { value: seconds };
  }
  return { value: trimmed };
}

/** A form with a save button around the boxes of one value. */
function SaveForm({
  isPending,
  onSubmit,
  children,
}: {
  readonly isPending: boolean;
  readonly onSubmit: () => void;
  readonly children: ReactNode;
}) {
  return (
    <Form onSubmit={onSubmit}>
      {children}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={isPending}>{t("common.save")}</SubmitButton>
      </div>
    </Form>
  );
}

/** A box for the kinds of value a person types: text, numbers and lengths of time. */
function TextEditor({ field, value, onCommit, problem, isPending = false }: ValueEditorProps) {
  const [text, setText] = useState(textOf(field, value));
  const [local, setLocal] = useState<string | undefined>();
  return (
    <SaveForm
      isPending={isPending}
      onSubmit={() => {
        const sent = sentFrom(field, text);
        setLocal("problem" in sent ? sent.problem : undefined);
        if ("value" in sent) {
          onCommit(sent.value);
        }
      }}
    >
      <TextField
        {...boxWords(field, local ?? problem)}
        kind={field.kind === "long-text" ? fieldKind.multiline : fieldKind.text}
        value={text}
        onChange={setText}
      />
    </SaveForm>
  );
}

/** The widest span of days a day box takes: the whole calendar. */
const CALENDAR = { min: "0001-01-01", max: "9999-12-31" };

/** A day, and for a day and time also the time of day. */
function MomentEditor({ field, value, onCommit, problem, isPending = false }: ValueEditorProps) {
  const withTime = field.kind === "date-time";
  const [day, setDay] = useState(typeof value === "string" ? value.slice(0, 10) : "");
  const [time, setTime] = useState(
    typeof value === "string" && withTime ? value.slice(11, 16) : "",
  );
  const sent = () => {
    if (day === "") {
      return null;
    }
    return withTime ? `${day}T${time === "" ? "00:00" : time}:00Z` : day;
  };
  return (
    <SaveForm
      isPending={isPending}
      onSubmit={() => {
        onCommit(sent());
      }}
    >
      <TextField
        {...boxWords(field, problem)}
        kind={fieldKind.text}
        date={CALENDAR}
        value={day}
        onChange={setDay}
      />
      {withTime ? (
        <TextField label={t("fields.time-of-day")} value={time} onChange={setTime} />
      ) : null}
    </SaveForm>
  );
}

function YesNoEditor({ field, value, onCommit }: ValueEditorProps) {
  return (
    <div {...partOf(part.stack)}>
      <Switch label={field.name} isSelected={value === true} onChange={onCommit} />
    </div>
  );
}

/** A list with the choices that are not retired, and the one the value has. */
function PickOneEditor({ field, value, onCommit }: ValueEditorProps) {
  const current = typeof value === "string" ? value : "";
  const choices = pickable(field, current === "" ? [] : [current]).map((choice) => ({
    value: choice.id,
    label: choice.name,
  }));
  const mayBeEmpty = field.rules.required !== true || current === "";
  return (
    <SelectField
      {...boxWords(field)}
      choices={[
        ...(mayBeEmpty ? [{ value: "", label: t("fields.nothing-picked") }] : []),
        ...choices,
      ]}
      value={current}
      onChange={(picked) => {
        onCommit(picked === "" ? null : picked);
      }}
    />
  );
}

/** Ticks for the choices that are not retired, and the ones the value has. */
function PickSeveralEditor({ field, value, onCommit, problem }: ValueEditorProps) {
  const current = Array.isArray(value)
    ? value.filter((each): each is string => typeof each === "string")
    : [];
  return (
    <fieldset {...partOf(part.choiceGroup)}>
      <legend {...partOf(part.choiceGroupLegend)}>{field.name}</legend>
      {pickable(field, current).map((choice) => (
        <Checkbox
          key={choice.id}
          label={choice.name}
          isSelected={current.includes(choice.id)}
          onChange={(on) => {
            const next = on
              ? [...current, choice.id]
              : current.filter((each) => each !== choice.id);
            onCommit(next.length === 0 ? null : next);
          }}
        />
      ))}
      {problem === undefined ? null : <p {...partOf(part.fieldError)}>{problem}</p>}
    </fieldset>
  );
}

/** People, groups and issues are shown, and can be taken away; picking them comes with the issue page. */
function ReferenceEditor({ field, value, onCommit, isPending = false }: ValueEditorProps) {
  const hasValue = value !== undefined && value !== null && field.rules.required !== true;
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.fieldLabel)}>{field.name}</p>
      <ValueView field={field} value={value} />
      {hasValue ? (
        <div {...partOf(part.row)}>
          <Button
            look={buttonLook.quiet}
            isPending={isPending}
            onPress={() => {
              onCommit(null);
            }}
          >
            {t("fields.take-away")}
          </Button>
        </div>
      ) : null}
    </div>
  );
}

/** One editor for each kind of value, so a new kind cannot be forgotten. */
const editors: Readonly<Record<KindWord, (props: ValueEditorProps) => ReactNode>> = {
  text: TextEditor,
  "long-text": TextEditor,
  "web-address": TextEditor,
  "whole-number": TextEditor,
  decimal: TextEditor,
  duration: TextEditor,
  "yes-no": YesNoEditor,
  date: MomentEditor,
  "date-time": MomentEditor,
  "pick-one": PickOneEditor,
  "pick-several": PickSeveralEditor,
  person: ReferenceEditor,
  people: ReferenceEditor,
  group: ReferenceEditor,
  issue: ReferenceEditor,
};

/**
 * The editor of the value of an own field (CF-01): a box for what is typed, a list for *Pick one*,
 * ticks for *Pick several*, a switch for yes or no. People, groups and issues are shown, and can be
 * taken away; picking them comes with the people pickers of the issue page. The server decides
 * what is allowed; a refusal comes back as the sentence of the rule.
 */
export function ValueEditor(props: ValueEditorProps) {
  const Editor = editors[props.field.kind];
  return <Editor {...props} />;
}
