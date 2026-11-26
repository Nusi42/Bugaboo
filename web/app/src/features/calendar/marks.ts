import { issueAddress } from "../../shared/paths.ts";
import { projectPageAddress, projectPages } from "../../shared/project-pages.ts";
import type { Release } from "../releases/index.ts";
import type { GoalMark } from "./calendar-data.ts";
import type { Window } from "./span-days.ts";

/** What stands on a day without being dragged (M10-Q8): a goal on its *Due date*, a release on its *Release date*. */
export interface Mark {
  /** What it is: the diamond of a goal or of a release says which in words, never by its shape alone. */
  readonly kind: "goal" | "release";
  readonly key: string;
  /** The Issue ID of a goal; nothing for a release. */
  readonly id: string | undefined;
  /** The title of a goal, the name of a release; none when the reader may not read the title of a goal. */
  readonly name: string | undefined;
  /** Where it opens: the issue page of a goal, the page *Releases* of the project of a release. */
  readonly href: string;
}

/** The goals and releases that stand in the window, by the day they stand on, goals first. */
export function marksOf(
  goals: readonly GoalMark[],
  releases: readonly Release[],
  window: Pick<Window, "first" | "last">,
): ReadonlyMap<string, readonly Mark[]> {
  const days = new Map<string, Mark[]>();
  const put = (day: string, mark: Mark): void => {
    days.set(day, [...(days.get(day) ?? []), mark]);
  };
  for (const goal of goals) {
    put(goal.day, {
      kind: "goal",
      key: `goal:${goal.issue}`,
      id: goal.readable_id,
      name: goal.title ?? undefined,
      href: issueAddress(goal.readable_id),
    });
  }
  for (const release of releases) {
    const day = release.releaseDate;
    if (day !== null && release.state !== "retired" && day >= window.first && day <= window.last) {
      put(day, {
        kind: "release",
        key: `release:${release.id}`,
        id: undefined,
        name: release.name,
        href: projectPageAddress(projectPages.releases, release.project.short_name),
      });
    }
  }
  return days;
}
