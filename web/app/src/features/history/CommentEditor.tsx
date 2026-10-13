import {
  newEditorId,
  releaseEditLock,
  sendHeartbeat,
  startHeartbeats,
  takeEditLock,
  type EditWhat,
} from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Form,
  MarkdownEditor,
  SubmitButton,
  partOf,
  part,
} from "@bugaboo/ui";
import { useEffect, useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { markdownLabels } from "../../shared/markdown-labels.ts";
import { refreshHistory } from "../../shared/query-keys.ts";

/** No image from another host in a comment being written: the hosts come with the issue page. */
const noImageHosts: readonly string[] = [];

type LockState =
  | { readonly status: "waiting" }
  | { readonly status: "held" }
  | { readonly status: "mine"; readonly token: string }
  | { readonly status: "failed"; readonly error: unknown };

/** The part of a comment that is edited under a lock: its text. */
function whatOf(comment: string): EditWhat {
  return { record: { kind: "comment", id: comment }, part: "text" };
}

/**
 * Takes the lock on the text of a comment when the editor opens, keeps it with the heartbeat while
 * the editor is open, and lets go of it when the editor closes (ADR-0019).
 */
function useCommentLock(comment: string): LockState {
  const [state, setState] = useState<LockState>({ status: "waiting" });
  useEffect(() => {
    const what = whatOf(comment);
    let open = true;
    let token: string | null = null;
    let stop = (): void => undefined;
    takeEditLock(api.client, what, newEditorId())
      .then((outcome) => {
        if (outcome.status !== "taken") {
          setState({ status: "held" });
          return;
        }
        const own = outcome.lock_token;
        token = own;
        if (open) {
          setState({ status: "mine", token: own });
          stop = startHeartbeats(() => sendHeartbeat(api.client, what, own));
        }
      })
      .catch((error: unknown) => {
        setState({ status: "failed", error });
      });
    return () => {
      open = false;
      stop();
      if (token !== null) {
        void releaseEditLock(api.client, what, token);
      }
    };
  }, [comment]);
  return state;
}

/** The text box with its buttons, once the editor holds the lock: saved with the token. */
function EditForm({
  comment,
  version,
  text,
  token,
  onClose,
}: {
  readonly comment: string;
  readonly version: number;
  readonly text: string;
  readonly token: string;
  readonly onClose: () => void;
}) {
  const [draft, setDraft] = useState(text);
  const save = api.query.useMutation("patch", "/api/v1/comments/{comment}", {
    onSuccess: () => {
      refreshHistory();
      onClose();
    },
  });
  return (
    <Form
      onSubmit={() => {
        save.mutate({
          params: { path: { comment } },
          body: { version, lock_token: token, text: draft },
        });
      }}
    >
      <MarkdownEditor
        label={t("comments.editing")}
        labels={markdownLabels()}
        value={draft}
        onChange={setDraft}
        imageHosts={noImageHosts}
      />
      {save.isError ? <Alert tone={alertTone.danger}>{messageOf(save.error)}</Alert> : null}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={save.isPending}>{t("comments.save")}</SubmitButton>
        <Button look={buttonLook.quiet} onPress={onClose}>
          {t("comments.cancel")}
        </Button>
      </div>
    </Form>
  );
}

/**
 * Edits the text of a comment (ISS-05, ADR-0019): under the lock, saved with its token and the
 * version the editor started from; every earlier text is kept by the server.
 */
export function CommentEditor(props: {
  readonly comment: string;
  readonly version: number;
  readonly text: string;
  readonly onClose: () => void;
}) {
  const lock = useCommentLock(props.comment);
  if (lock.status === "held") {
    return <Alert tone={alertTone.warning}>{t("comments.lock-held")}</Alert>;
  }
  if (lock.status === "failed") {
    return <Alert tone={alertTone.danger}>{messageOf(lock.error)}</Alert>;
  }
  return lock.status === "mine" ? <EditForm {...props} token={lock.token} /> : null;
}
