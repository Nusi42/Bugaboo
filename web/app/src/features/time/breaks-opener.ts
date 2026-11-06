import type { DaysProps } from "./sheet-props.ts";

/** The opener of the breaks window of a day: only a day with a record that may be changed has one. */
export function breaksOpener(
  props: Pick<DaysProps, "onBreaks" | "editable" | "dayOf">,
  date: string,
): (() => void) | undefined {
  const recorded = (props.dayOf(date)?.periods.length ?? 0) > 0;
  return recorded && props.editable(date)
    ? () => {
        props.onBreaks(date);
      }
    : undefined;
}
