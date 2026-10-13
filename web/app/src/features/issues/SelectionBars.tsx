import { selectionBars, type SelectionBarProps } from "./selection-bars.ts";

/** The bars other packages put under the list; each decides when it shows (while issues are ticked, or what it did). */
export function SelectionBars({ selection }: SelectionBarProps) {
  return (
    <>
      {selectionBars().map((entry) => (
        <entry.component key={entry.id} selection={selection} />
      ))}
    </>
  );
}
