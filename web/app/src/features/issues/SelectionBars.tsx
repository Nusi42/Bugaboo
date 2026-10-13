import { selectionBars, type SelectionBarProps } from "./selection-bars.ts";

/** The bars other packages put under the list, while some issues are ticked. */
export function SelectionBars({ selection }: SelectionBarProps) {
  if (selection.ids.length === 0) {
    return null;
  }
  return (
    <>
      {selectionBars().map((entry) => (
        <entry.component key={entry.id} selection={selection} />
      ))}
    </>
  );
}
