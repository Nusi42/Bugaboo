import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { useMe } from "../../shared/use-me.ts";
import { namedPerson } from "../../shared/person.ts";
import { refreshHistory } from "../../shared/query-keys.ts";

/** *Delete*, and *Undo* once it is deleted: a comment goes to the trash and comes back whole. */
function DeleteComment({ comment, mine }: { readonly comment: string; readonly mine: boolean }) {
  const remove = api.query.useMutation("delete", "/api/v1/comments/{comment}", {
    onSuccess: refreshHistory,
  });
  const restore = api.query.useMutation("post", "/api/v1/comments/{comment}/restore", {
    onSuccess: refreshHistory,
  });
  const params = { params: { path: { comment } } } as const;
  return (
    <>
      {/* On an own comment the short word stands on the button and its name says whose comment it is (WCAG 2.5.3):
          Edit and Delete then fit on one row on the smallest phone. */}
      <Button
        look={buttonLook.quiet}
        isPending={remove.isPending}
        aria-label={mine ? t("comments.delete") : t("comments.delete-other")}
        onPress={() => {
          remove.mutate(params);
        }}
      >
        {mine ? t("comments.delete-short") : t("comments.delete-other")}
      </Button>
      {remove.isSuccess ? (
        <Alert tone={alertTone.info}>
          <span>{t("comments.deleted")}</span>
          <Button
            look={buttonLook.quiet}
            isPending={restore.isPending}
            onPress={() => {
              restore.mutate(params);
            }}
          >
            {t("common.undo")}
          </Button>
        </Alert>
      ) : null}
      {remove.isError ? <Alert tone={alertTone.danger}>{messageOf(remove.error)}</Alert> : null}
    </>
  );
}

/** Whether the person who is signed in wrote the comment (a person shown only by role or as *Someone* is nobody's own). */
function isMine(comment: Schema<"HistoryEntryItem">, me: string | undefined): boolean {
  return me !== undefined && namedPerson(comment.by ?? undefined)?.id === me;
}

/** The buttons under a comment: what the reader may do with it. */
export function CommentActions({
  comment,
  editing,
  onEdit,
  onVersions,
  versionsShown,
}: {
  readonly comment: Schema<"HistoryEntryItem">;
  /** The editor is open: *Edit* would do nothing, so it is not offered. */
  readonly editing: boolean;
  readonly onEdit: () => void;
  readonly onVersions: () => void;
  readonly versionsShown: boolean;
}) {
  const id = comment.comment;
  const me = useMe();
  return (
    <div {...partOf(part.row)}>
      {comment.may_edit === true && !editing ? (
        <Button look={buttonLook.quiet} aria-label={t("comments.edit")} onPress={onEdit}>
          {t("comments.edit-short")}
        </Button>
      ) : null}
      {comment.may_delete === true && id !== undefined && id !== null ? (
        <DeleteComment comment={id} mine={isMine(comment, me?.id)} />
      ) : null}
      {comment.edited_at === undefined || comment.edited_at === null ? null : (
        <Button look={buttonLook.quiet} onPress={onVersions}>
          {versionsShown ? t("comments.versions-hide") : t("comments.versions")}
        </Button>
      )}
    </div>
  );
}
