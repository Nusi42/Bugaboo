import type { Schema } from "@bugaboo/api-client";

import { partOfAddress, paths, routeParam } from "../../shared/paths.ts";
import { readSearch } from "../../shared/search-address.ts";

/** What kind of thing the panel looks at: the words of the API. */
export type LookedAtKind = Schema<"LookingAtKind">;

/** One thing the panel looks at; the person can leave each out. */
export interface LookingAtItem {
  /** Tells the items apart: the kind and the value. */
  readonly key: string;
  readonly kind: LookedAtKind;
  /** An issue by its readable ID, a project by its short name, or the text of the search of a list. */
  readonly value: string;
}

function item(kind: LookedAtKind, value: string): LookingAtItem {
  return { key: `${kind}:${value}`, kind, value };
}

/** The short name of the project an issue ID starts with (`WEB` of `WEB-142`). */
function projectOfIssue(issue: string): string | undefined {
  const dash = issue.lastIndexOf("-");
  return dash > 0 ? issue.slice(0, dash) : undefined;
}

/** The start of every address of a project, like `/p/`. */
const PROJECT_PREFIX = paths.project.slice(0, paths.project.indexOf("$"));

/** The project of an address under a project (`/p/WEB/settings`), if it is one. */
function projectInAddress(pathname: string): string | undefined {
  if (!pathname.startsWith(PROJECT_PREFIX)) {
    return undefined;
  }
  const word = pathname.slice(PROJECT_PREFIX.length).split("/")[0];
  return word === undefined || word === "" ? undefined : decodeURIComponent(word);
}

/**
 * What the panel looks at when it opens at this address (UX §4.12): the issue of an issue page, the
 * project of a list, the issue open beside it and the search of the list. The person removes what
 * they do not want the assistant to look at.
 */
export function lookingAtOf(pathname: string, search: object): readonly LookingAtItem[] {
  const issue = partOfAddress(paths.issue, routeParam.issue, pathname);
  if (issue !== undefined) {
    return [item("issue", issue)];
  }
  const state = readSearch({ pathname, search });
  const items: LookingAtItem[] = [];
  const project = state.project ?? projectInAddress(pathname);
  if (project !== undefined) {
    items.push(item("project", project));
  }
  if (state.text !== "") {
    items.push(item("search", state.text));
  }
  if (state.open !== undefined) {
    items.push(item("issue", state.open));
  }
  return items;
}

/**
 * The project of the place, which says whether AI counts there: the project of a list, a page of a
 * project, or the project an issue ID starts with. None for a page that belongs to no project.
 */
export function projectOf(pathname: string, search: object): string | undefined {
  const issue = partOfAddress(paths.issue, routeParam.issue, pathname);
  if (issue !== undefined) {
    return projectOfIssue(issue);
  }
  return readSearch({ pathname, search }).project ?? projectInAddress(pathname);
}

/** The items without the one the person left out. */
export function withoutItem(
  items: readonly LookingAtItem[],
  key: string,
): readonly LookingAtItem[] {
  return items.filter((each) => each.key !== key);
}
