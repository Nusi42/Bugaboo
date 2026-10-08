/** How the choices of a picker look: a scrolling list of plain rows, or a column of bordered boxes. */
export const pickerLook = { list: "list", boxed: "boxed" } as const;

/** One of the looks of {@link pickerLook}. */
export type PickerLook = (typeof pickerLook)[keyof typeof pickerLook];
