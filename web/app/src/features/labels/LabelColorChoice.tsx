import type { ChoiceColorName } from "@bugaboo/ui";

import { ColorChoice } from "../../shared/ColorChoice.tsx";
import { colorChoiceLook } from "../../shared/color-choice-look.ts";

/**
 * The color of a label, chosen with the square the label will have beside each name (M5-F149), on the form that
 * creates a label and on the one that changes it.
 */
export function LabelColorChoice(props: {
  readonly legend: string;
  readonly value: ChoiceColorName | undefined;
  readonly onChange: (color: ChoiceColorName) => void;
}) {
  return <ColorChoice {...props} look={colorChoiceLook.square} />;
}
