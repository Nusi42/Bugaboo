import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useState } from "react";

import { editedThing, LockedEditor as Locked } from "../../shared/editing/index.ts";
import { DraftEditor } from "./DraftEditor.tsx";
import { whatOf } from "./layouts.ts";

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
  const [savedAt, setSavedAt] = useState<number | undefined>();
  const version = editor.layout?.version;
  return (
    <Locked what={whatOf(layout)} thing={editedThing.layout} lost={t("layouts.lost")}>
      {(lock) => (
        <DraftEditor
          key={version}
          editor={editor}
          lock={lock}
          saved={savedAt !== undefined && savedAt === version}
          onSaved={setSavedAt}
        />
      )}
    </Locked>
  );
}
