import { t } from "@bugaboo/messages";
import { useState } from "react";

import { editedThing, LockedEditor } from "../../shared/editing/index.ts";
import type { Draft } from "./draft-model.ts";
import { DraftWorkflow } from "./DraftWorkflow.tsx";
import { whatOf, type WorkflowEditor } from "./workflows.ts";

/**
 * The editor of a workflow that exists: the lock is taken when it opens and let go when it closes.
 * *Publish* changes the version, so the draft starts again from what was published (and the editor
 * remembers that this version is the one that was published); the lock is not given up, the screen
 * asks for it again. A draft that a refused publish left is offered again once the workflow is
 * read as it is now.
 */
export function LockedWorkflow({
  editor,
  workflow,
}: {
  readonly editor: WorkflowEditor;
  readonly workflow: string;
}) {
  const [publishedAt, setPublishedAt] = useState<number | undefined>();
  const [left, setLeft] = useState<Draft | undefined>();
  const version = editor.workflow?.version;
  return (
    <LockedEditor what={whatOf(workflow)} thing={editedThing.workflow} lost={t("workflows.lost")}>
      {(lock) => (
        <DraftWorkflow
          key={version}
          editor={editor}
          lock={lock}
          published={publishedAt !== undefined && publishedAt === version}
          onPublished={setPublishedAt}
          left={left}
          onLeft={setLeft}
        />
      )}
    </LockedEditor>
  );
}
