import { stretchesWords } from "../bar-words.ts";
import type { GroupLine } from "../lines.ts";
import { useTimeline } from "../timeline-context.ts";
import { hatchedStretches } from "./hatched-words.ts";

/**
 * The days without work of the person of a group under *Group by person* (M8-F90): hatched across
 * the chart from its first day, and their words for screen readers. Only a person shown by name
 * has a row, and the server sends only the days the reader may know.
 */
export function usePersonDays(line: GroupLine) {
  const timeline = useTimeline();
  const days =
    line.key.type === "person" && line.key.value.shown === "name"
      ? timeline.personDaysOf(line.key.value.id)
      : [];
  const stretches = hatchedStretches(days, timeline.span.first, true);
  return { stretches, words: stretchesWords(stretches, timeline.span.first) };
}
