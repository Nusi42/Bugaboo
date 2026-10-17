import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { useCreateWorkflow, type EditorQuery, type WorkflowEditor } from "./workflows.ts";

/** What the place uses now, in words: the workflow of another place, or the starting workflow. */
function usedNow(editor: WorkflowEditor): string {
  const from = editor.inherited_from;
  const name = editor.name;
  if (from === null || from === undefined) {
    return t("workflows.uses-starting", { name });
  }
  const where = from.whole_organization
    ? t("workflows.uses-organization", { name })
    : from.project === null || from.project === undefined
      ? t("workflows.uses-hidden", { name })
      : t("workflows.uses-project", { name, project: from.project.name });
  return from.issue_type === null || from.issue_type === undefined
    ? where
    : `${where} ${t("workflows.uses-type", { type: from.issue_type.name })}`;
}

/**
 * A place that has no workflow of its own: what it uses now, and *Customize* that creates one as a
 * copy of it, so the person can change it without touching the place it comes from.
 */
export function CustomizeWorkflow({
  editor,
  query,
}: {
  readonly editor: WorkflowEditor;
  readonly query: EditorQuery;
}) {
  const create = useCreateWorkflow(query);
  const project = editor.project;
  return (
    <div {...partOf(part.stack)}>
      <p>{usedNow(editor)}</p>
      <div {...partOf(part.row)}>
        <Button
          look={buttonLook.primary}
          isPending={create.isPending}
          onPress={() => {
            create.mutate();
          }}
        >
          {project === null || project === undefined
            ? t("workflows.customize-organization")
            : t("workflows.customize-project", { project: project.name })}
        </Button>
      </div>
      {create.isError ? <Alert tone={alertTone.danger}>{messageOf(create.error)}</Alert> : null}
    </div>
  );
}
