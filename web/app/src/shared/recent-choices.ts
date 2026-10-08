import { readStoredJson, storageKey, writeStored } from "@bugaboo/ui";
import { type SubjectKind } from "./person.ts";

/** Remembers in this browser what the person picked last, so a picker can offer it first (UX §6). */
/** How many choices of each kind are remembered. */
const REMEMBERED = 8;

/** The IDs picked last, the latest first. Only IDs are kept: no name is stored in the browser. */
function read(): Readonly<Record<SubjectKind, readonly string[]>> {
  const stored = readStoredJson(storageKey.recentChoices);
  const pick = (kind: SubjectKind): readonly string[] => {
    const ids: unknown =
      typeof stored === "object" && stored !== null ? Reflect.get(stored, kind) : [];
    return Array.isArray(ids) ? ids.filter((id): id is string => typeof id === "string") : [];
  };
  return { person: pick("person"), group: pick("group") };
}

/** Remembers a pick: it goes to the front. */
export function rememberChoice(kind: SubjectKind, id: string): void {
  const now = read();
  const next = {
    ...now,
    [kind]: [id, ...now[kind].filter((known) => known !== id)].slice(0, REMEMBERED),
  };
  writeStored(storageKey.recentChoices, JSON.stringify(next));
}

/** The items with what the person picked last in front, and the rest in the order they came. */
export function recentFirst<Item extends { readonly id: string }>(
  kind: SubjectKind,
  items: readonly Item[],
): readonly Item[] {
  const recent = read()[kind];
  const rank = (item: Item): number => {
    const at = recent.indexOf(item.id);
    return at < 0 ? recent.length : at;
  };
  return items.toSorted((first, second) => rank(first) - rank(second));
}
