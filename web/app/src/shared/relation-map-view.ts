/**
 * What the address of a relation map says besides its start (UX §2.1, §4.15): how many steps away
 * it walks and which kinds of relation it shows. Written once for the map of an issue, the map of a
 * search and their tools.
 */

/**
 * How many steps away a relation map walks: 1, 2 or 3, or none for *Everything*, which is the most
 * the search language walks (M10-Q11); the server knows that number, the screen does not.
 */
export type StepsAway = 1 | 2 | 3 | undefined;

/** The steps a person can choose, nearest first; the last is *Everything*. */
export const stepChoices: readonly StepsAway[] = [1, 2, 3, undefined];

/**
 * Which relations a map walks and draws (*Kinds shown*): the kinds of relation turned off by their
 * IDs, and whether *part of* is on. Nothing turned off is every kind, also one made later.
 */
export interface KindsShown {
  readonly hidden: readonly string[];
  readonly partOf: boolean;
}

/** Every kind shown, *part of* too: how a map starts. */
export const everyKind: KindsShown = { hidden: [], partOf: true };

/** The names of the parts of the address of a map. */
export const mapParam = { steps: "steps", hide: "hide", partOf: "parts" } as const;

/** What separates the kinds turned off in the address. */
const listSeparator = ",";

/** What the address writes for *part of* turned off. */
const partOfOff = "0";

/** The kinds shown that the parts `hide` and `parts` of an address say. */
export function kindsShownOf(hide: string | undefined, parts: string | undefined): KindsShown {
  const hidden = hide === undefined || hide === "" ? [] : hide.split(listSeparator);
  return { hidden, partOf: parts !== partOfOff };
}

/** The parts of an address that say the kinds shown: only what differs from every kind. */
export function kindsShownParts(shown: KindsShown): { hide?: string; parts?: string } {
  return {
    ...(shown.hidden.length === 0 ? {} : { [mapParam.hide]: shown.hidden.join(listSeparator) }),
    ...(shown.partOf ? {} : { [mapParam.partOf]: partOfOff }),
  };
}
