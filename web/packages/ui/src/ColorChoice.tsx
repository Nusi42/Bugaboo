import { useId } from "react";

import type { ChoiceColorName } from "./FieldValue.tsx";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

/** Words of the HTML standard, not text for people. */
const radio = "radio";

export interface ColorChoiceProps {
  /** The visible name of the group, already in the person's language. */
  readonly legend: string;
  /** The colors, each with its name in the person's language. */
  readonly colors: readonly { readonly value: ChoiceColorName; readonly label: string }[];
  readonly value: ChoiceColorName;
  /** The sample text on each swatch. */
  readonly sample: string;
  readonly onChange: (value: ChoiceColorName) => void;
}

/** One color out of the category colors of the theme, shown as swatches that sit side by side. */
export function ColorChoice({ legend, colors, value, sample, onChange }: ColorChoiceProps) {
  const group = useId();
  return (
    <fieldset {...partOf(part.choiceGroup)}>
      <legend {...partOf(part.choiceGroupLegend)}>{legend}</legend>
      <div {...partOf(part.choiceGroupChoices)}>
        {colors.map((color) => (
          <label key={color.value} {...partOf(part.choiceGroupChoice)}>
            <input
              {...partOf(part.choiceGroupInput)}
              type={radio}
              name={group}
              value={color.value}
              aria-label={color.label}
              checked={value === color.value}
              onChange={() => {
                onChange(color.value);
              }}
            />
            <span {...partOf(part.fieldSwatch, { variants: [color.value] })} aria-hidden>
              {sample}
            </span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}
