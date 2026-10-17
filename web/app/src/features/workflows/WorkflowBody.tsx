import { t } from "@bugaboo/messages";
import { TextField, partOf, part } from "@bugaboo/ui";

import { renameWorkflow, type Draft } from "./draft-model.ts";
import { DrawingView } from "./DrawingView.tsx";
import { StatusList } from "./StatusList.tsx";
import type { Edit } from "./use-draft-workflow.ts";
import { useView } from "./use-view.ts";
import { ViewChoice } from "./ViewChoice.tsx";
import type { WorkflowEditor } from "./workflows.ts";

export interface WorkflowBodyProps {
  readonly editor: WorkflowEditor;
  readonly draft: Draft;
  readonly edit: Edit;
  /** The editing is off: the person does not hold the lock. The choice of the view still works. */
  readonly disabled: boolean;
}

/**
 * The choice of the view, the name of the workflow, and the statuses and status changes in the view
 * that was chosen. Without the lock everything but the choice of the view is switched off.
 */
export function WorkflowBody({ editor, draft, edit, disabled }: WorkflowBodyProps) {
  const view = useView();
  return (
    <>
      <ViewChoice view={view} />
      <fieldset disabled={disabled} {...partOf(part.workflowEditor)}>
        <TextField
          label={t("workflows.name")}
          value={draft.name}
          onChange={(name) => {
            edit((before) => renameWorkflow(before, name));
          }}
        />
        {view.drawing ? (
          <DrawingView editor={editor} draft={draft} edit={edit} />
        ) : (
          <StatusList editor={editor} draft={draft} edit={edit} />
        )}
      </fieldset>
    </>
  );
}
