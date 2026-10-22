import { t } from "@bugaboo/messages";
import { Alert, alertTone } from "@bugaboo/ui";

import { SaveBar, shownSaveError, type EditLock } from "../../shared/editing/index.ts";
import { statusNamedTwice } from "./draft-model.ts";
import { PublishDialog } from "./PublishDialog.tsx";
import type { DraftWorkflowState } from "./use-draft-workflow.ts";
import { useNameKey } from "./use-name-key.ts";

export interface PublishBarProps {
  readonly state: DraftWorkflowState;
  readonly lock: EditLock;
  /** The workflow on screen is the one that was just published. */
  readonly published: boolean;
}

/** *Publish* and *Throw my changes away* with what happened to the last publish, and the question of *Publish*. */
export function PublishBar({ state, lock, published }: PublishBarProps) {
  const { publish } = state;
  const keyOf = useNameKey();
  // Two statuses with one name are refused by the server; the person is told which one, here.
  const twice = keyOf === undefined ? undefined : statusNamedTwice(state.draft, keyOf);
  return (
    <>
      {twice === undefined ? null : (
        <Alert tone={alertTone.danger}>{t("workflows.publish-name-twice", { name: twice })}</Alert>
      )}
      <SaveBar
        dirty={state.dirty}
        canSave={state.mine && twice === undefined}
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
