import { t } from "@bugaboo/messages";
import { Alert, alertTone, buttonLook, ConfirmDialog, partOf, part } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { useAskFirst } from "../../shared/ask-first.tsx";
import { messageOf } from "../../shared/errors.ts";
import { refreshAccess } from "../../shared/query-keys.ts";
import type { GroupLine } from "./groups.ts";

export interface DeleteGroupProps {
  readonly group: GroupLine;
  /** How many people are in it: the dialog says what happens to them. */
  readonly members: number;
  /** Called once the group is gone. */
  readonly onDeleted: () => void;
}

/**
 * *Delete the group*: a dialog that says exactly what happens (UX principle 7). It cannot be taken
 * back with *Undo*: the rows that gave the group a role go with it.
 */
export function DeleteGroup({ group, members, onDeleted }: DeleteGroupProps) {
  const remove = api.query.useMutation("delete", "/api/v1/groups/{group_id}", {
    onSuccess: () => {
      refreshAccess();
      onDeleted();
    },
  });
  const { ask, dialog } = useAskFirst();
  return (
    <div {...partOf(part.stack)}>
      {dialog}
      {remove.isError ? <Alert tone={alertTone.danger}>{messageOf(remove.error)}</Alert> : null}
      <div {...partOf(part.row)}>
        <ConfirmDialog
          triggerLabel={t("groups.delete")}
          triggerName={t("groups.delete-name", { name: group.name })}
          title={t("groups.delete-title", { name: group.name })}
          text={t("groups.delete-text", { members })}
          confirmLabel={t("groups.delete")}
          cancelLabel={t("common.cancel")}
          confirmLook={buttonLook.danger}
          onConfirm={() => {
            // The group may be what gives the person their own way to manage roles.
            void ask({ kind: "delete-group", group: group.id }).then((go) => {
              if (go) {
                remove.mutate({ params: { path: { group_id: group.id } } });
              }
            });
          }}
        />
      </div>
    </div>
  );
}
