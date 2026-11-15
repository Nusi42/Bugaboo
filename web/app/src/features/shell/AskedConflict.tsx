import { part, partOf } from "@bugaboo/ui";

// Straight from their files: the rest of the editing code is not part of the first page (PERF-04).
import { ConflictNote } from "../../shared/editing/ConflictNote.tsx";
import { useAskedConflict } from "../../shared/editing/conflict-prompt.ts";

/**
 * A clash of a screen with no place of its own to ask in (a bar of the timeline, UX §4.7): it stands
 * where the offers to undo stand, until it is answered.
 */
export function AskedConflict() {
  const asked = useAskedConflict();
  return asked === null ? null : (
    <div {...partOf(part.undoHostPrompt)}>
      <ConflictNote conflict={asked} />
    </div>
  );
}
