import type { PartChoices } from "@bugaboo/ui";
import type { NameKey } from "@bugaboo/wasm";

import { exactName, readsAlike } from "../../shared/same-name.ts";
import { useNameKey } from "../../shared/use-name-key.ts";
import { relationKindLinesOf, type RelationKindLine } from "../relation-kinds/index.ts";
import type { MapLine } from "./layout.ts";

/**
 * How a line of the relation map looks (UX §4.15). The kinds of relation are rows an administrator
 * can change, so a look is never tied to a name: it follows what the kind is. A kind with a date
 * effect is a dependency (*blocks*: orange, with an arrow); *part of* is violet towards the parent;
 * a kind that reads alike from both ends (*related to*) has no direction and no arrow; the other
 * kinds take the remaining looks in the order the kinds are offered in (*duplicate of* dashed grey,
 * *causes* red). The words of every line are drawn too, so a color is never the only signal (R5).
 */
export type LineLook = Exclude<
  NonNullable<PartChoices<"map-line">["variants"]>[number],
  false | undefined
>;

/** The looks the other kinds take in turn, in the order of the kinds. */
const otherLooks = ["other-1", "other-2"] as const satisfies readonly LineLook[];

/** The look of each kind of relation by its ID, and the look of *part of*. */
export interface Looks {
  readonly ofKind: (id: string) => LineLook;
  readonly partOf: LineLook;
}

/** The look of *part of*, the sub-issues and their parents. */
const partOfLook: LineLook = "part-of";

/** What decides the looks: the kinds in the order they are offered in, and the rule of the same name. */
export function looksOf(kinds: readonly RelationKindLine[], keyOf: NameKey): Looks {
  const looks = new Map<string, LineLook>();
  let others = 0;
  for (const kind of kinds) {
    if (kind.dateEffect !== "none") {
      looks.set(kind.id, "dependency");
    } else if (readsAlike(kind, keyOf)) {
      looks.set(kind.id, "alike");
    } else {
      looks.set(kind.id, otherLooks[others % otherLooks.length] ?? "other-1");
      others += 1;
    }
  }
  return { ofKind: (id) => looks.get(id) ?? "alike", partOf: partOfLook };
}

/** The look of a line of the map. */
export function lookOfLine(line: MapLine, looks: Looks): LineLook {
  const kind = line.relation_kind;
  return kind === undefined || kind === null ? looks.partOf : looks.ofKind(kind);
}

/** Whether a line of this look has a direction, and so an arrow. */
export function hasDirection(look: LineLook): boolean {
  return look !== "alike";
}

/** The pages of the kinds of relation read so far. */
export type KindPages = Parameters<typeof relationKindLinesOf>[0];

/** The kinds of relation in the order they are offered in, and the look of each. */
export function useLooks(pages: KindPages): {
  readonly kinds: readonly RelationKindLine[];
  readonly looks: Looks;
} {
  const kinds = relationKindLinesOf(pages);
  return { kinds, looks: looksOf(kinds, useNameKey() ?? exactName) };
}
