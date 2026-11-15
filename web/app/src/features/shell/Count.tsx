import { KeptRoom, type PartAttributes } from "@bugaboo/ui";

import { widestCount } from "../../shared/unread-notifications.ts";

/**
 * The number on a menu entry or a tab of the bar (unread notifications): nothing without a count, and where the entry
 * counts but there is no number to show (none, or none read yet) the room of the widest one, unseen, so the number comes
 * and goes without narrowing the words beside it (UX §6.4).
 */
export function Count({
  count,
  look,
}: {
  readonly count: string | undefined;
  readonly look: PartAttributes;
}) {
  if (count === undefined) {
    return null;
  }
  return count === "" ? (
    <KeptRoom>
      <span {...look}>{widestCount()}</span>
    </KeptRoom>
  ) : (
    <span {...look}>{count}</span>
  );
}
