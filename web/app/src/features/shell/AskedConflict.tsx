import { part, partOf } from "@bugaboo/ui";
import { useEffect } from "react";

// Straight from their files: the rest of the editing code is not part of the first page (PERF-04).
import { ConflictNote } from "../../shared/editing/ConflictNote.tsx";
import { useAskedConflict } from "../../shared/editing/conflict-prompt.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";

/**
 * A clash of a screen with no place of its own to ask in (a bar of the timeline, UX §4.7): it stands
 * where the offers to undo stand, until it is answered. While it stands, the offers beside it do
 * not run out: one that went would move its buttons under a finger (UX §6.4).
 */
export function AskedConflict() {
  const asked = useAskedConflict();
  const shows = asked !== null;
  useEffect(() => (shows ? undoStore.holdAll() : undefined), [shows]);
  return asked === null ? null : (
    <div {...partOf(part.undoHostPrompt)}>
      <ConflictNote conflict={asked} />
    </div>
  );
}
