import { t } from "@bugaboo/messages";
import { Announce, part, partOf } from "@bugaboo/ui";

import { AskFor } from "../issues/index.ts";
import { announcementOf } from "./announce.ts";
import { BoardGrid } from "./BoardGrid.tsx";
import { BoardContext } from "./holding.ts";
import { PhoneBoard } from "./PhoneBoard.tsx";
import { useBoardParts, type BoardPartsInput } from "./use-board-parts.ts";
import type { OwnOf } from "./CardFields.tsx";

export interface BoardBodyProps extends BoardPartsInput {
  readonly ownOf: OwnOf;
}

/**
 * The board that was read, with its moves: the columns and cards, the window of *Ask for* while a move
 * asks for something, the words that say how the keys work, and what a screen reader is told while a
 * card is picked up (UX §4.6, TEST-A11Y).
 */
export function BoardBody({ ownOf, ...input }: BoardBodyProps) {
  const { state, rowName } = useBoardParts(input);
  const { moves, holding, keysId } = state;
  const grid = { model: input.model, own: ownOf, rowName };
  return (
    <BoardContext.Provider value={state}>
      <p id={keysId} {...partOf(part.visuallyHidden)}>
        {t("board.keys")}
      </p>
      <Announce>{announcementOf(holding, state.columns)}</Announce>
      {input.phone ? <PhoneBoard {...grid} /> : <BoardGrid {...grid} />}
      {moves.asking === null ? null : (
        <AskFor
          issue={moves.asking.card.issue}
          change={moves.asking.change}
          changes={{ ask: moves.ask }}
          onClose={moves.closeAsking}
        />
      )}
    </BoardContext.Provider>
  );
}
