import { useState } from "react";

import type { EditLock } from "../../shared/editing/index.ts";
import { draftOf, sameDraft, type Draft } from "./draft-model.ts";
import { usePublishFlow, type PublishFlow } from "./use-publish.ts";
import type { WorkflowEditor } from "./workflows.ts";

/** Changes the draft: the screen hands over a function from the draft as it is to the draft it becomes. */
export type Edit = (change: (draft: Draft) => Draft) => void;

/** What the screen of the editor gets: the draft, what it is changed with, and *Publish*. */
export interface DraftWorkflowState {
  readonly draft: Draft;
  readonly edit: Edit;
  /** Whether the editing is on: the person holds the lock. */
  readonly mine: boolean;
  readonly dirty: boolean;
  readonly publish: PublishFlow;
  readonly discard: () => void;
  /** Puts the changes of a draft that was left in place of the draft on screen. */
  readonly restore: (draft: Draft) => void;
}

/** What the state needs besides the workflow: the lock, and what happens after a publish. */
interface Needs {
  readonly editor: WorkflowEditor;
  readonly lock: EditLock;
  readonly onPublished: (version: number) => void;
  readonly onLeft: (draft: Draft | undefined) => void;
}

/**
 * The state of the editor of one workflow: the draft that holds everything the person changed until
 * *Publish*, and the publish, which is made on the version the editor looked at and under the lock
 * the person holds. After a publish the server has let go of the lock, so it is asked for again.
 */
export function useDraftWorkflow({ editor, lock, onPublished, onLeft }: Needs): DraftWorkflowState {
  const [draft, setDraft] = useState(() => draftOf(editor));
  const publish = usePublishFlow({ editor, lock, draft, onPublished, onLeft });
  return {
    draft,
    edit: setDraft,
    mine: lock.state.phase === "mine",
    dirty: !sameDraft(draft, draftOf(editor)),
    publish,
    discard: () => {
      setDraft(draftOf(editor));
    },
    restore: setDraft,
  };
}
