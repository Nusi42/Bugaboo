import { InPaneHead, paneHeadSlot } from "@bugaboo/ui";

import { IssueLegend } from "../issues/index.ts";
import { BoardTools, type BoardToolsProps } from "./BoardTools.tsx";

/**
 * The head of the pane of a board: the tools and the one Legend of the page (the colors of the rows
 * and the edges of the priorities, as the list and the timeline have them). A phone has the Legend in
 * the row of its search bar.
 */
export function BoardHead(props: BoardToolsProps) {
  return (
    <InPaneHead slot={paneHeadSlot.view}>
      <BoardTools {...props} />
      {props.phone ? null : <IssueLegend />}
    </InPaneHead>
  );
}
