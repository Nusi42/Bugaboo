/**
 * How a color is shown in the choice: as the fill of a chip with its name in the color's words (a tag, a choice of an
 * own field), or as the clear square of a label beside its name (M5-F149), so the choice shows what the label will have.
 */
export const colorChoiceLook = { fill: "fill", square: "square" } as const;
export type ColorChoiceLook = (typeof colorChoiceLook)[keyof typeof colorChoiceLook];
