import { cardLimitMax } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import type { BoardOptions } from "../../shared/list-view.ts";

/** The least a card limit may be; the most is the server's (`cardLimitMax`). */
const limitLeast = 1;

/** One card limit as the options keep it: a column by its key (none for *No value*) and its number. */
export type CardLimit = NonNullable<BoardOptions["card_limits"]>[number];

/** What a box of a card limit holds: a number written out, or nothing for no limit. */
export function limitText(limits: readonly CardLimit[], column: string | null): string {
  const found = limits.find((each) => each.column === column);
  return found === undefined ? "" : String(found.limit);
}

/** The words of a box that holds no number from 1 to the highest limit; none for an empty box or a number the server takes. */
export function limitProblem(text: string): string | undefined {
  if (text.trim() === "") {
    return undefined;
  }
  const number = Number(text);
  const fits = /^\d+$/u.test(text.trim()) && number >= limitLeast && number <= cardLimitMax;
  return fits ? undefined : t("board.limit-invalid", { most: cardLimitMax });
}

/** The card limits the boxes hold, for the columns they belong to, as the options keep them. */
export function limitsOf(
  columns: readonly (string | null)[],
  texts: Readonly<Record<string, string>>,
): readonly CardLimit[] {
  return columns.flatMap((column) => {
    const text = (texts[column ?? ""] ?? "").trim();
    return text === "" ? [] : [{ column, limit: Number(text) }];
  });
}
