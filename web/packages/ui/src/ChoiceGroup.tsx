import { useId, useLayoutEffect, useRef, useState, type ReactNode } from "react";

import { partOf } from "./parts.ts";
import { choiceGroupLook, type ChoiceGroupLook } from "./props.ts";
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
  /** The chosen one; none while nothing is chosen yet (a form where the server picks when nobody does). */
  readonly value: Value | undefined;
  readonly onChange: (value: Value) => void;
  /** How a choice shows its name, next to the input; the usual label by default. */
  readonly renderLabel?: (choice: Choice<Value>) => ReactNode;
  /**
   * How the choices are laid out besides the usual (side by side while they fit on one line, else under each other):
   * a grid that wraps, several in a row, for many small choices such as the colors.
   */
  readonly look?: ChoiceGroupLook;
}

/** Follows the width of the row (and the fonts arriving): true while its choices, side by side, do not fit on one line. */
function useStackedWhenWrapped() {
  const row = useRef<HTMLDivElement>(null);
  const [stacked, setStacked] = useState(false);
  useLayoutEffect(() => {
    const node = row.current;
    // Measured as if side by side, whatever they are now: the variant is taken off and put back before anything is drawn.
    const fit = (): void => {
      if (node === null) {
        return;
      }
      const { variant } = node.dataset;
      delete node.dataset["variant"];
      const tops = new Set<number>();
      for (const child of node.children) {
        tops.add(Math.round(child.getBoundingClientRect().top));
      }
      if (variant !== undefined) {
        node.dataset["variant"] = variant;
      }
      setStacked(tops.size > 1);
    };
    fit();
    const watch = new ResizeObserver(fit);
    if (node !== null) {
      watch.observe(node);
    }
    void document.fonts.ready.then(fit);
    return () => {
      watch.disconnect();
    };
  }, []);
  return { row, stacked };
}

type OptionsProps<Value extends string> = Pick<
  ChoiceGroupProps<Value>,
  "choices" | "value" | "onChange" | "renderLabel"
> & {
  /** The name that makes the radios one group. */
  readonly group: string;
};

/** The choices as radios, each with its label. */
function Options<Value extends string>({
  choices,
  value,
  onChange,
  renderLabel,
  group,
}: OptionsProps<Value>) {
  return choices.map((choice) => (
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
  ));
}

/** The usual layout: side by side while they fit on one line, measured, else under each other. */
function MeasuredChoices<Value extends string>(props: OptionsProps<Value>) {
  const { row, stacked } = useStackedWhenWrapped();
  return (
    <div ref={row} {...partOf(part.choiceGroupChoices, { variants: [stacked && "stacked"] })}>
      <Options {...props} />
    </div>
  );
}

/**
 * One choice out of a few, shown as buttons that sit side by side while they all fit on one line, and under each
 * other, as wide as the screen, when they do not: themes, spacing and the like. With the grid look the choices stand
 * in a grid that wraps instead (the colors), and nothing is measured.
 */
export function ChoiceGroup<Value extends string>({
  legend,
  look,
  ...options
}: ChoiceGroupProps<Value>) {
  const group = useId();
  return (
    <fieldset {...partOf(part.choiceGroup)}>
      <legend {...partOf(part.choiceGroupLegend)}>{legend}</legend>
      {look === choiceGroupLook.grid ? (
        <div {...partOf(part.choiceGroupChoices, { variants: [look] })}>
          <Options {...options} group={group} />
        </div>
      ) : (
        <MeasuredChoices {...options} group={group} />
      )}
    </fieldset>
  );
}
