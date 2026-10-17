import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";

import type { WorkflowsOpen } from "../../shared/paths.ts";
import { IssueTypeChooser } from "../issue-types/index.ts";
import { QueryGate } from "../shell/index.ts";
import { CustomizeWorkflow } from "./CustomizeWorkflow.tsx";
import { LockedWorkflow } from "./LockedWorkflow.tsx";
import { UseInherited } from "./UseInherited.tsx";
import { useWorkflowEditor, type EditorQuery, type WorkflowEditor } from "./workflows.ts";

export interface WorkflowsCardProps {
  /** The project, by its short name; nothing for the whole organization. */
  readonly project: string | undefined;
  /** The heading of the card. */
  readonly heading: string;
  readonly open: WorkflowsOpen;
  /** Opens another issue type. */
  readonly onOpen: (open: WorkflowsOpen) => void;
}

/** The editor of the workflow that is open, once it is read: to customize, or to change. */
function Open({ editor, query }: { readonly editor: WorkflowEditor; readonly query: EditorQuery }) {
  const slot = editor.workflow;
  if (slot === null || slot === undefined) {
    return <CustomizeWorkflow editor={editor} query={query} />;
  }
  return (
    <>
      <LockedWorkflow key={slot.id} editor={editor} workflow={slot.id} />
      <UseInherited slot={slot} />
    </>
  );
}

/**
 * The editor of the workflows of a place (WF-02, UX §5.4): which issue type, and then the workflow,
 * or the way to create one. The same card serves a project and the whole organization.
 */
export function WorkflowsCard({ project, heading, open, onOpen }: WorkflowsCardProps) {
  const query: EditorQuery = { project, issueType: open.type };
  const editor = useWorkflowEditor(query);
  return (
    <Card heading={heading} level={1}>
      <div {...partOf(part.stack)}>
        <p {...partOf(part.muted)}>{t("workflows.intro")}</p>
        <IssueTypeChooser
          label={t("issue-field.issue-type")}
          type={open.type}
          onChange={(type) => {
            onOpen({ type });
          }}
        />
        <QueryGate error={editor.error} data={editor.data}>
          {(found) => <Open key={open.type ?? ""} editor={found} query={query} />}
        </QueryGate>
      </div>
    </Card>
  );
}
