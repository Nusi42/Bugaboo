import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import type { Draft } from "./draft-model.ts";
import { AddStatus } from "./AddStatus.tsx";
import { StatusBlock } from "./StatusBlock.tsx";
import type { Edit } from "./use-draft-workflow.ts";
import type { WorkflowEditor } from "./workflows.ts";

export interface StatusListProps {
  readonly editor: WorkflowEditor;
  readonly draft: Draft;
  readonly edit: Edit;
}

/**
 * The plain list (UX §5.4): each status with its group, the statuses it can go to and the words
 * of the buttons. It is the default view and the only one on a phone.
 */
export function StatusList({ editor, draft, edit }: StatusListProps) {
  return (
    <section aria-label={t("workflows.statuses")} {...partOf(part.stack)}>
      <h2 {...partOf(part.cardHeading)}>{t("workflows.statuses")}</h2>
      <p {...partOf(part.muted)}>{t("workflows.statuses-help")}</p>
      <ol {...partOf(part.workflowStatuses)}>
        {draft.statuses.map((status, at) => (
          <StatusBlock
            key={status.id}
            editor={editor}
            draft={draft}
            status={status}
            first={at === 0}
            last={at === draft.statuses.length - 1}
            edit={edit}
          />
        ))}
      </ol>
      <AddStatus draft={draft} edit={edit} />
    </section>
  );
}
