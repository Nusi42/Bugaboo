import type { Schema } from "@bugaboo/api-client";

import { partOfAddress, paths, routeParam } from "../../shared/paths.ts";
import { projectInAddress } from "../../shared/assistant-place.ts";
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

/** The items without the one the person left out. */
export function withoutItem(
  items: readonly LookingAtItem[],
  key: string,
): readonly LookingAtItem[] {
  return items.filter((each) => each.key !== key);
}
