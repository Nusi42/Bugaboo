import { t } from "@bugaboo/messages";

import type { BoardColumn } from "./board-data.ts";
import { columnName, keyOf } from "./card-moves.ts";
import { heldBy, type Holding } from "./holding.ts";

/**
 * What a screen reader is told while a card is picked up by keys (UX §4.6, TEST-A11Y): that it is in
 * hand, and for the column the arrows chose whether it can go there or why not. Nothing for a drag
 * with the pointer, and nothing while no card is in hand.
 */
export function announcementOf(holding: Holding, columns: readonly BoardColumn[]): string {
  const { held, card } = holding;
  const column = held === null ? undefined : columns[held.target];
  if (held === null || card === undefined || column === undefined || held.by !== heldBy.keys) {
    return "";
  }
  const key = keyOf(card);
  const offer = holding.offerOf(held.target);
  const note = offer.reason ?? t("board.announce-can-go", { key });
  return `${t("board.announce-picked", { key })} ${t("board.announce-column", { column: columnName(column), note })}`;
}
