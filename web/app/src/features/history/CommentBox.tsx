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

import { lockCanBeRetaken, type LongText, type LostReason } from "../../shared/editing/index.ts";
import { messageOf } from "../../shared/errors.ts";
import { markdownLabels } from "../../shared/markdown-labels.ts";

/** No image from another host in a comment being written: the hosts come with the issue page. */
const noImageHosts: readonly string[] = [];

/** The sentence that says why the text was not saved, for each reason (the messages of the server). */
const lostMessage = {
  "ran-out": "error.edit.lock-lost",
  "taken-over": "error.edit.taken-over",
  "version-changed": "error.edit.version-changed",
} as const satisfies Record<LostReason, string>;

/** What the person typed, kept as a draft and shown while someone else holds the lock. */
export function KeptText({ text }: { readonly text: string }) {
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.muted)}>{t("editing.text-kept")}</p>
      <p {...partOf(part.text)}>{text}</p>
    </div>
  );
}

/** Why the text was not saved, with *Try again* where taking the lock again can help. */
function LostNote({
  reason,
  onRetry,
}: {
  readonly reason: LostReason;
  readonly onRetry: () => void;
}) {
  return (
    <Alert tone={alertTone.warning}>
      <div {...partOf(part.issueNote)}>
        <span>{t(lostMessage[reason])}</span>
        {lockCanBeRetaken(reason) ? (
          <Button onPress={onRetry}>{t("common.try-again")}</Button>
        ) : null}
      </div>
    </Alert>
  );
}

/** The box with its buttons, and what went wrong with the last save. */
export function CommentBox({
  long,
  onClose,
}: {
  readonly long: LongText;
  readonly onClose: () => void;
}) {
  const { phase } = long;
  const saving = phase.kind === "saving";
  return (
    <Form onSubmit={long.save}>
      <MarkdownEditor
        label={t("comments.editing")}
        labels={markdownLabels()}
        value={long.text}
        onChange={long.type}
        imageHosts={noImageHosts}
      />
      {long.problem === null || long.problem === undefined ? null : (
        <Alert tone={alertTone.danger}>{messageOf(long.problem)}</Alert>
      )}
      {phase.kind === "lost" ? <LostNote reason={phase.reason} onRetry={long.save} /> : null}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={saving}>{t("common.save")}</SubmitButton>
        <Button look={buttonLook.quiet} onPress={long.cancel}>
          {t("common.cancel")}
        </Button>
        <Button look={buttonLook.quiet} onPress={onClose}>
          {t("common.close")}
        </Button>
      </div>
      <p {...partOf(part.muted)}>{t("editing.text-kept-on-close")}</p>
    </Form>
  );
}
