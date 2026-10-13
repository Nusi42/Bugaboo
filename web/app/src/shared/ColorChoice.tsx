import { choiceColorWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { ChoiceGroup, partOf, part, type ChoiceColorName } from "@bugaboo/ui";

const colorChoices = choiceColorWords.map((word) => ({
  value: word,
  label: t(`choice-color.${word}`),
}));

export interface ColorChoiceProps {
  /** What the choice is called for people who read it with a screen reader: "Color of login". */
  readonly legend: string;
  readonly value: ChoiceColorName;
  readonly onChange: (color: ChoiceColorName) => void;
}

/**
 * One of the twelve category colors of the theme (UX §7.1), each shown as a swatch with its name.
 * The color of a choice of an own field, of a tag and of a label is chosen the same way.
 */
export function ColorChoice({ legend, value, onChange }: ColorChoiceProps) {
  return (
    <ChoiceGroup
      legend={legend}
      choices={colorChoices}
      value={value}
      renderLabel={(color) => (
        <span {...partOf(part.fieldSwatch, { variants: [color.value] })}>{color.label}</span>
      )}
      onChange={onChange}
    />
  );
}
