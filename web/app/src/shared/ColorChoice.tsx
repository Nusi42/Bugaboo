import { choiceColorWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Icon, iconName, iconSize, partOf, part, type ChoiceColorName } from "@bugaboo/ui";
import { useId } from "react";

/** Words of the HTML standard, not text for people. */
const radio = "radio";

export interface ColorChoiceProps {
  /** What the choice is called for people who read it with a screen reader: "Color of login". */
  readonly legend: string;
  /** The chosen color; none while nothing is chosen (a new label gets the next color in turn). */
  readonly value: ChoiceColorName | undefined;
  readonly onChange: (color: ChoiceColorName) => void;
}

/**
 * One of the twelve category colors of the theme (UX §7.1), each a small swatch with its name, several in a row in a
 * grid that wraps. The chosen one has a ring and a tick, so it is told by more than its color (also under forced
 * colors). The color of a choice of an own field, of a tag and of a label is chosen here, the same way.
 */
export function ColorChoice({ legend, value, onChange }: ColorChoiceProps) {
  const group = useId();
  return (
    <fieldset {...partOf(part.colorChoice)}>
      <legend {...partOf(part.colorChoiceLegend)}>{legend}</legend>
      <div {...partOf(part.colorChoiceGrid)}>
        {choiceColorWords.map((color) => (
          <label key={color} {...partOf(part.colorChoiceOption)}>
            <input
              {...partOf(part.colorChoiceInput)}
              type={radio}
              name={group}
              value={color}
              checked={value === color}
              onChange={() => {
                onChange(color);
              }}
            />
            <span {...partOf(part.fieldSwatch, { variants: [color] })}>
              <span {...partOf(part.colorChoiceTick)}>
                <Icon name={iconName.check} size={iconSize.small} />
              </span>
              {t(`choice-color.${color}`)}
            </span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}
