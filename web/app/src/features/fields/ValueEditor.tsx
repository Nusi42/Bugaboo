import { t } from "@bugaboo/messages";
import { Checkbox, SelectField, Switch, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { boxWords, labelOf } from "./box-words.ts";
import type { OwnField, KindWord, Sent } from "./fields.ts";
import { MomentEditor } from "./MomentEditor.tsx";
import { ReferenceEditor } from "./ReferenceEditor.tsx";
import { DurationEditor, TextEditor } from "./TextEditor.tsx";

export type { Sent } from "./fields.ts";

export interface ValueEditorProps {
  readonly field: OwnField;
  /** The value now, as the API words it; nothing when there is none. */
  readonly value: unknown;
  /** Sends a new value; `null` takes the value away. */
  readonly onCommit: (value: Sent) => void;
  /** What the server said was wrong with the last value, in words. */
  readonly problem?: string | undefined;
  readonly isPending?: boolean;
  /** The project of the item (its short name), for the people a field can point to. */
  readonly project?: string | undefined;
  /** The issue (its ID), when the item is one: only people who may see it can be chosen. */
  readonly issue?: string | undefined;
  /** The ID of the type of the issue that is being made, while there is no issue yet. */
  readonly issueType?: string | undefined;
  /**
   * The editor sits in a form that is sent as a whole (the new-issue form): there is no *Save*,
   * and every change is handed on at once.
   */
  readonly live?: boolean;
  /**
   * The editor sits in a name-and-value row of the Details: the name is the row's, there is no
   * *Save*, and what is typed is kept when the person leaves the box or presses Enter (R7).
   */
  readonly inRow?: boolean;
}

/** The choices a person can pick: those not retired, and the ones the value has now. */
function pickable(field: OwnField, current: readonly string[]) {
  return field.choices.filter((choice) => !choice.retired || current.includes(choice.id));
}

function YesNoEditor({ field, value, onCommit, inRow = false }: ValueEditorProps) {
  return (
    <div {...partOf(part.stack)}>
      <Switch
        hideLabel={inRow}
        label={field.name}
        isSelected={value === true}
        onChange={onCommit}
      />
    </div>
  );
}

/** A list with the choices that are not retired, and the one the value has. */
function PickOneEditor({ field, value, onCommit, problem, inRow = false }: ValueEditorProps) {
  const current = typeof value === "string" ? value : "";
  const choices = pickable(field, current === "" ? [] : [current]).map((choice) => ({
    value: choice.id,
    label: choice.name,
  }));
  const mayBeEmpty = field.rules.required !== true || current === "";
  return (
    <SelectField
      {...boxWords(field, problem, inRow)}
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
function PickSeveralEditor({ field, value, onCommit, problem, inRow = false }: ValueEditorProps) {
  const current = Array.isArray(value)
    ? value.filter((each): each is string => typeof each === "string")
    : [];
  return (
    <fieldset {...partOf(part.choiceGroup)}>
      <legend {...partOf(inRow ? part.visuallyHidden : part.choiceGroupLegend)}>
        {labelOf(field)}
      </legend>
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

/** One editor for each kind of value, so a new kind cannot be forgotten. */
const editors: Readonly<Record<KindWord, (props: ValueEditorProps) => ReactNode>> = {
  text: TextEditor,
  "long-text": TextEditor,
  "web-address": TextEditor,
  "whole-number": TextEditor,
  decimal: TextEditor,
  duration: DurationEditor,
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
