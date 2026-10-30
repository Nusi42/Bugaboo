import type { ChoiceColorName } from "./FieldValue.tsx";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface LabelChipProps {
  /** The name of the label. */
  readonly text: string;
  /** The color of the label: a small square carries it. */
  readonly color?: ChoiceColorName | undefined;
}

/** The small square in the color of a label (UX §7.5); it only decorates, the name beside it says what it is. */
export function LabelMark({ color }: { readonly color?: ChoiceColorName | undefined }) {
  return (
    <span
      {...partOf(part.labelMark, { variants: color === undefined ? [] : [color] })}
      aria-hidden
    />
  );
}

/** A label in a list (UX §7.5): a small square in its color and its name as plain text, not a tinted pill. */
export function LabelChip({ text, color }: LabelChipProps) {
  return (
    <span {...partOf(part.labelChip)}>
      <LabelMark color={color} />
      {text}
    </span>
  );
}
