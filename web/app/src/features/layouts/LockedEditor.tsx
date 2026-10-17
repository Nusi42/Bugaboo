import type { Schema } from "@bugaboo/api-client";
import { useState } from "react";

import { DraftEditor } from "./DraftEditor.tsx";
import { EditNotices } from "./EditNotices.tsx";
import { useLayoutLock } from "./use-layout-lock.ts";

/**
 * The editor of a layout that exists: the lock is taken when it opens and let go when it closes.
 * A save changes the version, so the draft starts again from what was saved (and the editor
 * remembers that this version is the one that was saved); the lock is not given up, the screen
 * asks for it again.
 */
export function LockedEditor({
  editor,
  layout,
}: {
  readonly editor: Schema<"LayoutEditorBody">;
  readonly layout: string;
}) {
  const lock = useLayoutLock(layout);
  const [savedAt, setSavedAt] = useState<number | undefined>();
  const version = editor.layout?.version;
  return (
    <>
      <EditNotices lock={lock} />
      <DraftEditor
        key={version}
        editor={editor}
        lock={lock}
        saved={savedAt !== undefined && savedAt === version}
        onSaved={setSavedAt}
      />
    </>
  );
}
