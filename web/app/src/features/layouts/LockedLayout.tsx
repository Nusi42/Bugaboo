import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { editedThing, LockedEditor } from "../../shared/editing/index.ts";
import { DraftEditor } from "./DraftEditor.tsx";
import { whatOf } from "./layouts.ts";

/** The editor of a layout that exists, under the edit lock of its place. */
export function LockedLayout({
  editor,
  layout,
}: {
  readonly editor: Schema<"LayoutEditorBody">;
  readonly layout: string;
}) {
  const version = editor.layout?.version;
  return (
    <LockedEditor
      what={whatOf(layout)}
      thing={editedThing.layout}
      lost={t("layouts.lost")}
      version={version}
    >
      {(lock, saving) => (
        <DraftEditor
          key={version}
          editor={editor}
          lock={lock}
          saved={saving.saved}
          onSaved={saving.onSaved}
        />
      )}
    </LockedEditor>
  );
}
