import { keepsTextAsDraft, type Schema } from "@bugaboo/api-client";
import { useState } from "react";

import type { EditLock } from "../../shared/editing/index.ts";
import { codeOf } from "../../shared/errors.ts";
import type { Draft } from "./draft-model.ts";
import {
  usePublishPreview,
  usePublishWorkflow,
  type PublishPreview,
  type WorkflowEditor,
} from "./workflows.ts";

/** Where the issues of one removed status go: the key of the removed status and the key it goes to. */
export interface Move {
  readonly from: string;
  readonly to: string;
}

/** What the screen gets of *Publish*: to start it, to answer its question, and how it stands. */
export interface PublishFlow {
  /** The preview or the publish is on its way. */
  readonly pending: boolean;
  /** Why the preview or the publish did not work. */
  readonly error: unknown;
  /** The question *Where do the issues go?*, while it is asked. */
  readonly asking: PublishPreview | null;
  readonly start: () => void;
  readonly confirm: (moves: readonly Move[]) => void;
  readonly cancel: () => void;
}

/** What *Publish* needs from the screen. */
export interface PublishArgs {
  readonly editor: WorkflowEditor;
  readonly lock: EditLock;
  readonly draft: Draft;
  /** The publish worked; says the version it made. */
  readonly onPublished: (version: number) => void;
  /** The publish was refused in a way that keeps the changes: they are handed on to be offered again. */
  readonly onLeft: (draft: Draft) => void;
}

/** Whether any issue is in a removed status, so the person has to say where it goes. */
function asksWhere(preview: PublishPreview): boolean {
  return preview.dropped.some((each) => each.issues > 0);
}

/**
 * The publish itself: made under the lock the person holds, on the version the editor looked at,
 * with the places the issues of the removed statuses go to.
 */
function usePublishSend(
  { editor, lock, draft, onPublished, onLeft }: PublishArgs,
  close: () => void,
) {
  const publish = usePublishWorkflow();
  const slot = editor.workflow;
  const send = (moves: readonly Move[]): void => {
    if (slot === null || slot === undefined || lock.state.phase !== "mine") {
      return;
    }
    publish.mutate(
      { workflow: slot.id, version: slot.version, lockToken: lock.state.token, draft, moves },
      {
        onSuccess: (done: Schema<"PublishedBody">) => {
          close();
          onPublished(done.workflow.version);
        },
        onError: (error: unknown) => {
          close();
          if (keepsTextAsDraft(codeOf(error))) {
            onLeft(draft);
          }
        },
        onSettled: (_done: unknown, error: unknown) => {
          lock.settled(error);
        },
      },
    );
  };
  return { send, pending: publish.isPending, error: publish.error };
}

/**
 * *Publish* (UX §5.4): first asks the server which statuses the draft drops and how many issues
 * are in them; only when there are issues does it ask where they go.
 */
export function usePublishFlow(args: PublishArgs): PublishFlow {
  const preview = usePublishPreview();
  const [asking, setAsking] = useState<PublishPreview | null>(null);
  const { send, pending, error } = usePublishSend(args, () => {
    setAsking(null);
  });
  const slot = args.editor.workflow;
  // Without the lock the question is closed, and forgotten: when the lock is back it would else
  // open again with answers about a workflow that may have changed. The note of the lost lock says
  // what happened.
  if (args.lock.state.phase !== "mine" && asking !== null) {
    setAsking(null);
  }
  return {
    pending: preview.isPending || pending,
    error: preview.error ?? error,
    asking,
    start: () => {
      if (slot === null || slot === undefined) {
        return;
      }
      preview.mutate(
        { workflow: slot.id, draft: args.draft },
        {
          onSuccess: (found: PublishPreview) => {
            if (asksWhere(found)) {
              setAsking(found);
            } else {
              send([]);
            }
          },
        },
      );
    },
    confirm: send,
    cancel: () => {
      setAsking(null);
    },
  };
}
