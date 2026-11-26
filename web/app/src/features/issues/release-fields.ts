import type { ReleaseRef } from "../releases/index.ts";

/** The two fields that name releases, by their words in the layout. */
export const releaseField = {
  found: "found-in-release",
  planned: "planned-for-release",
} as const;

export type ReleaseWord = (typeof releaseField)[keyof typeof releaseField];

/** The one member a change of the releases of a field sets: it is a change of an issue and a member of the form. */
export type ReleaseValues =
  | { readonly foundInRelease: readonly ReleaseRef[] }
  | { readonly plannedForRelease: ReleaseRef | null };

/** What each of the two fields is called, whether it holds several releases, and what changing it sends. */
export const releaseFields: Readonly<
  Record<
    ReleaseWord,
    {
      readonly name: "issue-field.found-in-release" | "issue-field.planned-for-release";
      readonly several: boolean;
      readonly values: (releases: readonly ReleaseRef[]) => ReleaseValues;
    }
  >
> = {
  [releaseField.found]: {
    name: "issue-field.found-in-release",
    several: true,
    values: (releases) => ({ foundInRelease: releases }),
  },
  [releaseField.planned]: {
    name: "issue-field.planned-for-release",
    several: false,
    values: (releases) => ({ plannedForRelease: releases[0] ?? null }),
  },
};

/** What names releases: an issue as the server shows it, or the form of a new issue in the same two members. */
interface Naming {
  readonly found_in_release?: readonly ReleaseRef[] | null | undefined;
  readonly planned_for_release?: ReleaseRef | null | undefined;
}

/** The releases something names in a field: the one rule for the issue page and the form. */
export function heldBy(source: Naming, word: ReleaseWord): readonly ReleaseRef[] {
  return word === releaseField.found
    ? (source.found_in_release ?? [])
    : [source.planned_for_release ?? []].flat();
}
