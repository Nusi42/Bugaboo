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

/**
 * One choice out of many, in the list of the browser: it works with a keyboard and a screen
 * reader as it is, and a phone shows its own picker. For few choices that fit on a line,
 * `ChoiceGroup` is better; this is for long lists such as time zones and pages.
 */
export function SelectField<Value extends string>({
  label,
  description,
  hideLabel = false,
  choices,
  value,
  onChange,
}: SelectFieldProps<Value>) {
  const id = useId();
  const select = useRef<HTMLSelectElement>(null);
  const helpId = `${id}-help`;
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
        {...partOf(part.selectField)}
        value={value}
        {...(description === undefined ? {} : { "aria-describedby": helpId })}
        onChange={() => {
          const chosen = choices.find((choice) => choice.value === select.current?.value);
          if (chosen !== undefined) {
            onChange(chosen.value);
          }
        }}
      >
        {choices.map((choice) => (
          <option key={choice.value} value={choice.value} disabled={choice.disabled === true}>
            {choice.label}
          </option>
        ))}
      </select>
    </div>
  );
}
