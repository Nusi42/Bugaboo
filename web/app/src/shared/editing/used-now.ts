import type { Schema } from "@bugaboo/api-client";

/** What a layout or a workflow of a place says it uses now, in the words of each kind. */
export interface UsedNowWords {
  /** The place has nothing of another place: the starting layout or workflow. */
  readonly starting: string;
  readonly organization: string;
  /** Another project's, one the person may not see. */
  readonly hidden: string;
  readonly project: (project: string) => string;
  /** Said after the place, when the one used is for one issue type only. */
  readonly type: (type: string) => string;
}

/**
 * What the place uses now, in words: the starting one, or the one of another place (the whole
 * organization, a project, with its issue type). The sentence is the same for a layout and a
 * workflow, only the words differ.
 */
export function usedNowText(
  from: Schema<"InheritedFromBody"> | null | undefined,
  words: UsedNowWords,
): string {
  if (from === null || from === undefined) {
    return words.starting;
  }
  const where = from.whole_organization
    ? words.organization
    : from.project === null || from.project === undefined
      ? words.hidden
      : words.project(from.project.name);
  return from.issue_type === null || from.issue_type === undefined
    ? where
    : `${where} ${words.type(from.issue_type.name)}`;
}
