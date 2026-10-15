import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshHistory } from "../../shared/query-keys.ts";

/** *Delete*, and *Undo* once it is deleted: a comment goes to the trash and comes back whole. */
function DeleteComment({ comment }: { readonly comment: string }) {
  const remove = api.query.useMutation("delete", "/api/v1/comments/{comment}", {
    onSuccess: refreshHistory,
  });
  const restore = api.query.useMutation("post", "/api/v1/comments/{comment}/restore", {
    onSuccess: refreshHistory,
  });
  const params = { params: { path: { comment } } } as const;
  return (
    <>
      <Button
        look={buttonLook.quiet}
        isPending={remove.isPending}
        onPress={() => {
          remove.mutate(params);
        }}
      >
        {t("comments.delete")}
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
            {t("comments.undo")}
          </Button>
        </Alert>
      ) : null}
      {remove.isError ? <Alert tone={alertTone.danger}>{messageOf(remove.error)}</Alert> : null}
    </>
  );
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
  return (
    <div {...partOf(part.row)}>
      {comment.may_edit === true && !editing ? (
        <Button look={buttonLook.quiet} onPress={onEdit}>
          {t("comments.edit")}
        </Button>
      ) : null}
      {comment.may_delete === true && id !== undefined && id !== null ? (
        <DeleteComment comment={id} />
      ) : null}
      {comment.edited_at === undefined || comment.edited_at === null ? null : (
        <Button look={buttonLook.quiet} onPress={onVersions}>
          {versionsShown ? t("comments.versions-hide") : t("comments.versions")}
        </Button>
      )}
    </div>
  );
}
