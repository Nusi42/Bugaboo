import { useReleases } from "../releases/index.ts";
import type { GoalMark } from "./calendar-data.ts";
import { marksOf } from "./marks.ts";
import type { Window } from "./span-days.ts";

/**
 * The marks of the days on screen: the due dates of the goals the server answers with the issues, and
 * the *Release date* of the releases offered in the project the calendar is of. A calendar of every
 * project has no releases to mark (the releases are read for one project).
 */
export function useMarks(
  within: string | undefined,
  goals: readonly GoalMark[],
  window: Pick<Window, "first" | "last">,
) {
  const releases = useReleases(within);
  return marksOf(goals, releases.releases, window);
}
