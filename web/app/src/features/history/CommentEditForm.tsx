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

import { api } from "../../shared/api.ts";
import { lostReasonOf } from "../../shared/editing/index.ts";
import { codeOf, messageOf } from "../../shared/errors.ts";
import { markdownLabels } from "../../shared/markdown-labels.ts";
import { refreshHistory } from "../../shared/query-keys.ts";

/** No image from another host in a comment being written: the hosts come with the issue page. */
const noImageHosts: readonly string[] = [];

/** Whether a refusal says the lock ran out or was taken over: taking it again can work, the text is kept. */
function lockCanBeTakenAgain(error: unknown): boolean {
  const reason = lostReasonOf(codeOf(error));
  return reason === "ran-out" || reason === "taken-over";
}

/** What was wrong with the save; when the lock is gone, *Try again* takes it again. */
function Refused({ error, onRetry }: { readonly error: unknown; readonly onRetry: () => void }) {
  return (
    <Alert tone={alertTone.danger}>
      <div {...partOf(part.issueNote)}>
        <span>{messageOf(error)}</span>
        {lockCanBeTakenAgain(error) ? (
          <Button onPress={onRetry}>{t("common.try-again")}</Button>
        ) : null}
      </div>
    </Alert>
  );
}

/** The text box with its buttons, once the editor holds the lock: saved with the token. */
interface CommentEditFormProps {
  readonly comment: string;
  readonly version: number;
  /** The text in the box; the editor keeps it, so it survives taking the lock again. */
  readonly draft: string;
  readonly onDraft: (text: string) => void;
  readonly token: string;
  readonly onClose: () => void;
  /** Takes the lock again, with the text as it is. */
  readonly onRetry: () => void;
}

export function CommentEditForm(props: CommentEditFormProps) {
  const { comment, version, draft, onDraft, token, onClose, onRetry } = props;
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
        onChange={onDraft}
        imageHosts={noImageHosts}
      />
      {save.isError ? <Refused error={save.error} onRetry={onRetry} /> : null}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={save.isPending}>{t("common.save")}</SubmitButton>
        <Button look={buttonLook.quiet} onPress={onClose}>
          {t("common.cancel")}
        </Button>
      </div>
    </Form>
  );
}
