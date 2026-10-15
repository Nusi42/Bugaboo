import { useId, useRef } from "react";
import { VisuallyHidden } from "react-aria-components";

import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface SelectChoice<Value extends string> {
  readonly value: Value;
  /** The visible name of the choice, already in the person's language. */
  readonly label: string;
  /** The choice is shown but cannot be picked. */
  readonly disabled?: boolean;
}

export interface SelectFieldProps<Value extends string> {
  /** The visible name of the field, already in the person's language. */
  readonly label: string;
  /** Help under the name. */
  readonly description?: string;
  /** A problem the server found with the value: shown under the field. */
  readonly errorMessage?: string | undefined;
  /**
   * The name is there for screen readers and voice control only, for a list of many fields that
   * each stand next to what they are about (a table of levels). Never the way to save room when
   * the name matters to everybody.
   */
  readonly hideLabel?: boolean;
  readonly choices: readonly SelectChoice<Value>[];
  readonly value: Value;
  readonly onChange: (value: Value) => void;
}

/** The choices of the list, as the browser's own options. */
function Options({ choices }: { readonly choices: readonly SelectChoice<string>[] }) {
  return choices.map((choice) => (
    <option key={choice.value} value={choice.value} disabled={choice.disabled === true}>
      {choice.label}
    </option>
  ));
}

/** What the server found wrong with the value, under the list. */
function ErrorNote({ id, text }: { readonly id: string; readonly text: string | undefined }) {
  return text === undefined ? null : (
    <span id={id} {...partOf(part.fieldError)}>
      {text}
    </span>
  );
}

/** The IDs of the notes under a field, for `aria-describedby`; nothing when there are none. */
function describedByOf(...ids: readonly (string | null)[]): string | undefined {
  const present = ids.filter((each) => each !== null);
  return present.length === 0 ? undefined : present.join(" ");
}

/**
 * One choice out of many, in the list of the browser: it works with a keyboard and a screen
 * reader as it is, and a phone shows its own picker. For few choices that fit on a line,
 * `ChoiceGroup` is better; this is for long lists such as time zones and pages.
 */
export function SelectField<Value extends string>({
  label,
  description,
  errorMessage,
  hideLabel = false,
  choices,
  value,
  onChange,
}: SelectFieldProps<Value>) {
  const id = useId();
  const select = useRef<HTMLSelectElement>(null);
  const helpId = `${id}-help`;
  const errorId = `${id}-error`;
  const describedBy = describedByOf(
    description === undefined ? null : helpId,
    errorMessage === undefined ? null : errorId,
  );
  return (
    <div {...partOf(part.field)}>
      <label {...partOf(part.fieldLabel)} htmlFor={id}>
        {hideLabel ? <VisuallyHidden>{label}</VisuallyHidden> : label}
      </label>
      {description === undefined ? null : (
        <span id={helpId} {...partOf(part.fieldDescription)}>
          {description}
        </span>
      )}
      <select
        ref={select}
        id={id}
        {...partOf(part.fieldInput)}
        value={value}
        {...(describedBy === undefined ? {} : { "aria-describedby": describedBy })}
        {...(errorMessage === undefined ? {} : { "aria-invalid": true })}
        onChange={() => {
          const chosen = choices.find((choice) => choice.value === select.current?.value);
          if (chosen !== undefined) {
            onChange(chosen.value);
          }
        }}
      >
        <Options choices={choices} />
      </select>
      <ErrorNote id={errorId} text={errorMessage} />
    </div>
  );
}
