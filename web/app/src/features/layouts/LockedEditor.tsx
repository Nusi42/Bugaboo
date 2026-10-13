import type { Schema } from "@bugaboo/api-client";

import { DraftEditor } from "./DraftEditor.tsx";
import { LockNotes } from "./LockNotes.tsx";
import { useLayoutLock } from "./use-layout-lock.ts";

/**
 * The editor of a layout that exists: the lock is taken when it opens and let go when it closes.
 * A save changes the version, so the draft starts again from what was saved; the lock is not
 * given up (the screen asks again).
 */
export function LockedEditor({
  editor,
  layout,
}: {
  readonly editor: Schema<"LayoutEditorBody">;
  readonly layout: string;
}) {
  const lock = useLayoutLock(layout);
  return (
    <>
      <LockNotes lock={lock} />
      <DraftEditor key={editor.layout?.version} editor={editor} lock={lock} />
    </>
  );
}
