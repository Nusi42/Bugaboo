import { t } from "@bugaboo/messages";

import { editedThing, LockedEditor } from "../../shared/editing/index.ts";
import { useLeftDraft } from "./left-draft.ts";
import { DraftWorkflow } from "./DraftWorkflow.tsx";
import { whatOf, type WorkflowEditor } from "./workflows.ts";

/**
 * The editor of a workflow that exists: the lock is taken when it opens and let go when it closes.
 * A draft that a refused publish left is offered again once the workflow is read as it is now.
 */
export function LockedWorkflow({
  editor,
  workflow,
}: {
  readonly editor: WorkflowEditor;
  readonly workflow: string;
}) {
  const version = editor.workflow?.version;
  const [left, setLeft] = useLeftDraft(workflow, version);
  return (
    <LockedEditor
      what={whatOf(workflow)}
      thing={editedThing.workflow}
      lost={t("workflows.lost")}
      version={version}
    >
      {(lock, saving) => (
        <DraftWorkflow
          key={version}
          editor={editor}
          lock={lock}
          published={saving.saved}
          onPublished={saving.onSaved}
          left={left}
          onLeft={setLeft}
        />
      )}
    </LockedEditor>
  );
}
