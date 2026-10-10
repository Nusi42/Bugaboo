import { useId, type ReactNode } from "react";

import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

/** Words of the HTML standard, not text for people. */
const radio = "radio";

export interface Choice<Value extends string> {
  readonly value: Value;
  /** The visible name of the choice, already in the person's language. */
  readonly label: string;
}

export interface ChoiceGroupProps<Value extends string> {
  /** The visible name of the group, already in the person's language. */
  readonly legend: string;
  readonly choices: readonly Choice<Value>[];
  readonly value: Value;
  readonly onChange: (value: Value) => void;
  /** How a choice shows its name, next to the input; the usual label by default. */
  readonly renderLabel?: (choice: Choice<Value>) => ReactNode;
}

/** One choice out of a few, shown as buttons that sit side by side: themes, spacing and the like. */
export function ChoiceGroup<Value extends string>({
  legend,
  choices,
  value,
  onChange,
  renderLabel,
}: ChoiceGroupProps<Value>) {
  const group = useId();
  return (
    <fieldset {...partOf(part.choiceGroup)}>
      <legend {...partOf(part.choiceGroupLegend)}>{legend}</legend>
      <div {...partOf(part.choiceGroupChoices)}>
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
            {renderLabel === undefined ? (
              <span {...partOf(part.choiceGroupLabel)}>{choice.label}</span>
            ) : (
              renderLabel(choice)
            )}
          </label>
        ))}
      </div>
    </fieldset>
  );
}
