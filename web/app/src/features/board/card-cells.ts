import { issueAddress } from "../../shared/paths.ts";
import { known } from "../../shared/present.ts";
import { issueCells } from "../issues/index.ts";
import type { BoardCard } from "./board-data.ts";
import { keyOf } from "./card-moves.ts";

/** The cells a card has like every row of an issue: its Issue ID and type, and its title, which opens the issue. */
export function cardCells(card: BoardCard) {
  const key = keyOf(card);
  return issueCells(
    {
      key,
      title: known(card.issue.title) ?? "",
      group: card.issue.status?.status_group,
      statusWord: card.issue.status?.name,
      type: card.issue.issue_type ?? undefined,
    },
    issueAddress(key),
  );
}
