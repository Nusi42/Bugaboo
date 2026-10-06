import { useId, useRef } from "react";

import styles from "./Field.module.css";
import selectStyles from "./SelectField.module.css";

export interface SelectChoice<Value extends string> {
  readonly value: Value;
  /** The visible name of the choice, already in the person's language. */
  readonly label: string;
}

export interface SelectFieldProps<Value extends string> {
  /** The visible name of the field, already in the person's language. */
  readonly label: string;
  /** Help under the name. */
  readonly description?: string;
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
  choices,
  value,
  onChange,
}: SelectFieldProps<Value>) {
  const id = useId();
  const select = useRef<HTMLSelectElement>(null);
  const helpId = `${id}-help`;
  return (
    <div className={styles.field}>
      <label className={styles.label} htmlFor={id}>
        {label}
      </label>
      {description === undefined ? null : (
        <span id={helpId} className={styles.description}>
          {description}
        </span>
      )}
      <select
        ref={select}
        id={id}
        className={selectStyles.select}
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
          <option key={choice.value} value={choice.value}>
            {choice.label}
          </option>
        ))}
      </select>
    </div>
  );
}
