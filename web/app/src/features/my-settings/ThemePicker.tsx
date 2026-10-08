import { useId } from "react";
import { partOf, part } from "@bugaboo/ui";

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
    <fieldset {...partOf(part.choiceGroup)}>
      <legend {...partOf(part.choiceGroupLegend)}>{legend}</legend>
      <div {...partOf(part.row, { variants: ["stretch"] })}>
        {choices.map((choice) => (
          <label key={choice.value} {...partOf(part.choiceGroupChoice)}>
            <input
              {...partOf(part.choiceGroupInput)}
              type={radio}
              name={group}
              value={choice.value}
              checked={value === choice.value}
              onChange={() => {
                onChange(choice.value);
              }}
            />
            <span {...partOf(part.themePickerTile)}>
              <ThemePreview theme={choice.value} />
              <span {...partOf(part.themePickerName)}>{choice.label}</span>
            </span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}
