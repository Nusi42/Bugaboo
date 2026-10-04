import { useId } from "react";

import styles from "./ThemePicker.module.css";
import { themeChoices, type ThemeChoice } from "./theme.ts";

export interface ThemePickerProps {
  /** The visible name of the group, already in the person's language. */
  readonly legend: string;
  /** The visible name of each choice, already in the person's language. */
  readonly labels: Readonly<Record<ThemeChoice, string>>;
  readonly value: ThemeChoice;
  readonly onChange: (choice: ThemeChoice) => void;
}

export function ThemePicker({ legend, labels, value, onChange }: ThemePickerProps) {
  const group = useId();
  return (
    <fieldset className={styles.group}>
      <legend className={styles.legend}>{legend}</legend>
      <div className={styles.choices}>
        {themeChoices.map((choice) => (
          <label key={choice} className={styles.choice}>
            <input
              className={styles.input}
              type="radio"
              name={group}
              value={choice}
              checked={value === choice}
              onChange={() => {
                onChange(choice);
              }}
            />
            <span className={styles.label}>{labels[choice]}</span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}
