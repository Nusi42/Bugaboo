import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { DraftNote, type EditLock } from "../../shared/editing/index.ts";
import type { Draft } from "./draft-model.ts";
import { PublishBar } from "./PublishBar.tsx";
import { TryIt } from "./TryIt.tsx";
import { useDraftWorkflow } from "./use-draft-workflow.ts";
import { WorkflowBody } from "./WorkflowBody.tsx";
import type { WorkflowEditor } from "./workflows.ts";

export interface DraftWorkflowProps {
  readonly editor: WorkflowEditor;
  readonly lock: EditLock;
  /** The workflow on screen is the one that was just published. */
  readonly published: boolean;
  /** A publish worked; says the version that was made. */
  readonly onPublished: (version: number) => void;
  /** The draft that a refused publish left, to offer again. */
  readonly left: Draft | undefined;
  readonly onLeft: (draft?: Draft) => void;
}

/**
 * The workflow being changed: its statuses with the status changes between them, as a list (and, on
 * a computer, as a drawing), and the rules of each status change. Everything is kept in the draft
 * until *Publish*. Without the lock the whole editor is switched off, and the draft stays for when
 * the lock comes back.
 */
export function DraftWorkflow(props: DraftWorkflowProps) {
  const { editor, lock, published, left, onLeft } = props;
  const state = useDraftWorkflow(props);
  return (
    <>
      {left === undefined ? null : (
        <DraftNote
          text={t("workflows.draft-changed")}
          take={{
            label: t("workflows.draft-restore"),
            onPress: () => {
              state.restore(left);
              onLeft();
            },
          }}
          drop={{
            label: t("editing.changes-drop"),
            onPress: () => {
              onLeft();
            },
          }}
        />
      )}
      <WorkflowBody editor={editor} draft={state.draft} edit={state.edit} disabled={!state.mine} />
      <p {...partOf(part.muted)}>{t("workflows.draft-note")}</p>
      <PublishBar state={state} lock={lock} published={published} />
      <TryIt fields={editor.fields} />
    </>
  );
}
