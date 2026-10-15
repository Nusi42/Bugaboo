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

import { editedThing } from "../../shared/editing/edited-thing.ts";
import {
  lockCanBeRetaken,
  type LongText,
  type LostReason,
  type Who,
} from "../../shared/editing/index.ts";
import { LostNote } from "../../shared/editing/LostNote.tsx";
import { messageOf } from "../../shared/errors.ts";
import { markdownLabels } from "../../shared/markdown-labels.ts";

/** No image from another host in a comment being written: the hosts come with the issue page. */
const noImageHosts: readonly string[] = [];

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
function Lost({
  reason,
  by,
  onRetry,
}: {
  readonly reason: LostReason;
  readonly by: Who | undefined;
  readonly onRetry: () => void;
}) {
  return (
    <LostNote reason={reason} by={by} thing={editedThing.comment}>
      {lockCanBeRetaken(reason) ? <Button onPress={onRetry}>{t("common.try-again")}</Button> : null}
    </LostNote>
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
      {phase.kind === "lost" ? (
        <Lost reason={phase.reason} by={phase.by} onRetry={long.save} />
      ) : null}
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
