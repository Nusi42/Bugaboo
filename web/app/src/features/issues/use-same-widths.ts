import { useState } from "react";

/** The widths of the columns, the same object while they are the same widths: what is counted from them is not counted again on every draw. */
export function useSameWidths(
  widths: Readonly<Record<string, number>>,
): Readonly<Record<string, number>> {
  const [same, setSame] = useState(widths);
  if (JSON.stringify(same) !== JSON.stringify(widths)) {
    setSame(widths);
  }
  return same;
}
