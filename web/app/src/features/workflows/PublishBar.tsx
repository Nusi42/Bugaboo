import { t } from "@bugaboo/messages";

import { SaveBar, shownSaveError, type EditLock } from "../../shared/editing/index.ts";
import { PublishDialog } from "./PublishDialog.tsx";
import type { DraftWorkflowState } from "./use-draft-workflow.ts";

export interface PublishBarProps {
  readonly state: DraftWorkflowState;
  readonly lock: EditLock;
  /** The workflow on screen is the one that was just published. */
  readonly published: boolean;
}

/** *Publish* and *Throw my changes away* with what happened to the last publish, and the question of *Publish*. */
export function PublishBar({ state, lock, published }: PublishBarProps) {
  const { publish } = state;
  return (
    <>
      <SaveBar
        dirty={state.dirty}
        canSave={state.mine}
        pending={publish.pending}
        saved={published}
        error={shownSaveError(publish.error, lock.state)}
        onSave={publish.start}
        onDiscard={state.discard}
        saveLabel={t("workflows.publish")}
      />
      {publish.asking === null ? null : (
        <PublishDialog
          preview={publish.asking}
          draft={state.draft}
          pending={publish.pending}
          onConfirm={publish.confirm}
          onCancel={publish.cancel}
        />
      )}
    </>
  );
}
