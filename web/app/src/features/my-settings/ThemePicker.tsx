import { useId } from "react";

import styles from "./ThemePicker.module.css";
import { ThemePreview } from "./ThemePreview.tsx";

/** Words of the HTML standard, not text for people. */
const radio = "radio";

export interface ThemeChoice {
  /** The ID of the theme. */
  readonly value: string;
  /** The name of the theme, already in the person's language. */
  readonly label: string;
}

export interface ThemePickerProps {
  /** The visible name of the group. */
  readonly legend: string;
  readonly choices: readonly ThemeChoice[];
  readonly value: string;
  readonly onChange: (value: string) => void;
}

/** One theme out of a few, each shown by a sample of how it looks (THM-02). */
export function ThemePicker({ legend, choices, value, onChange }: ThemePickerProps) {
  const group = useId();
  return (
    <fieldset className={styles.group}>
      <legend className={styles.legend}>{legend}</legend>
      <div className={styles.choices}>
        {choices.map((choice) => (
          <label key={choice.value} className={styles.choice}>
            <input
              className={styles.input}
              type={radio}
              name={group}
              value={choice.value}
              checked={value === choice.value}
              onChange={() => {
                onChange(choice.value);
              }}
            />
            <span className={styles.tile}>
              <ThemePreview theme={choice.value} />
              <span className={styles.name}>{choice.label}</span>
            </span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}
