import { t } from "@bugaboo/messages";

import { CustomizeEditable, usedNowText } from "../../shared/editing/index.ts";
import { useCreateWorkflow, type EditorQuery, type WorkflowEditor } from "./workflows.ts";

/** What the place uses now, in words: the workflow of another place, or the starting workflow. */
function usedNow(editor: WorkflowEditor): string {
  const name = editor.name;
  return usedNowText(editor.inherited_from, {
    starting: t("workflows.uses-starting", { name }),
    organization: t("workflows.uses-organization", { name }),
    hidden: t("workflows.uses-hidden", { name }),
    project: (project) => t("workflows.uses-project", { name, project }),
    type: (type) => t("workflows.uses-type", { type }),
  });
}

/** A place that has no workflow of its own: what it uses now, and *Customize*. */
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
    <CustomizeEditable
      used={usedNow(editor)}
      label={
        project === null || project === undefined
          ? t("workflows.customize-organization")
          : t("workflows.customize-project", { project: project.name })
      }
      create={{
        isPending: create.isPending,
        isError: create.isError,
        error: create.error,
        mutate: () => {
          create.mutate();
        },
      }}
    />
  );
}
