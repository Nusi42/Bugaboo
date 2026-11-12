import { useId } from "react";
import { partOf, part } from "@bugaboo/ui";

import { ThemePreview, type PreviewedOwn } from "./ThemePreview.tsx";

/** Words of the HTML standard, not text for people. */
const radio = "radio";

export interface ThemeChoice {
  /** The value the setting keeps: the ID of a built-in theme, or the choice of an own theme. */
  readonly value: string;
  /** The name of the theme, already in the person's language. */
  readonly label: string;
  /** The built-in theme the sample shows: the theme itself, or the one an own theme extends. */
  readonly look: string;
  /** The own theme the sample lays over it; none for a built-in theme. */
  readonly own?: PreviewedOwn | undefined;
}

export interface ThemePickerProps {
  /** The visible name of the group. */
  readonly legend: string;
  readonly choices: readonly ThemeChoice[];
  readonly value: string;
  readonly onChange: (value: string) => void;
}

/**
 * One theme out of a few, each shown by a sample of how it looks (THM-02): the built-in themes,
 * then the organization's themes for everyone and the person's own (THM-04).
 */
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
              <ThemePreview theme={choice.look} own={choice.own} />
              <span {...partOf(part.themePickerName)}>{choice.label}</span>
            </span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}
