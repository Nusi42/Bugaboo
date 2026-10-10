import { registerFollowersBlock } from "../features/follow/index.ts";
import { registerHistoryBlock } from "../features/history/index.ts";
import { registerRelationBlocks } from "../features/relations/index.ts";

/**
 * Puts the blocks of the issue page that sub-issues, related issues, history and followers add
 * (WP-07) into their slots (`features/issues/slots.ts`). Called once, when the routes are loaded.
 */
export function registerIssueBlocks(): void {
  registerRelationBlocks();
  registerHistoryBlock();
  registerFollowersBlock();
}
