import { choiceColorWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  ChoiceGroup,
  choiceGroupLook,
  Icon,
  iconName,
  iconSize,
  partOf,
  part,
  type ChoiceColorName,
} from "@bugaboo/ui";

const colorChoices = choiceColorWords.map((word) => ({
  value: word,
  label: t(`choice-color.${word}`),
}));

export interface ColorChoiceProps {
  /** What the choice is called for people who read it with a screen reader: "Color of login". */
  readonly legend: string;
  /** The chosen color; none while nothing is chosen (a new label gets the next color in turn). */
  readonly value: ChoiceColorName | undefined;
  readonly onChange: (color: ChoiceColorName) => void;
}

/**
 * One of the twelve choice colors of the theme (UX §7.1), each a small swatch with its name, several in a row in the
 * grid of a choice group. The chosen one has a ring and a tick, so it is told by more than its color (also under
 * forced colors). The color of a choice of an own field, of a tag and of a label is chosen here, the same way.
 */
export function ColorChoice({ legend, value, onChange }: ColorChoiceProps) {
  return (
    <ChoiceGroup
      legend={legend}
      choices={colorChoices}
      value={value}
      look={choiceGroupLook.grid}
      renderLabel={(color) => (
        <span {...partOf(part.fieldSwatch, { variants: [color.value] })}>
          <span {...partOf(part.fieldSwatchTick)}>
            <Icon name={iconName.check} size={iconSize.small} />
          </span>
          {color.label}
        </span>
      )}
      onChange={onChange}
    />
  );
}
