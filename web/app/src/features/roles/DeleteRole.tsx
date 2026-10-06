import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, ConfirmDialog } from "@bugaboo/ui";
import { useState } from "react";

import { codeOf, messageOf } from "../../shared/errors.ts";
import { ReplacementField } from "./ReplacementField.tsx";
import styles from "./Roles.module.css";
import { useDeleteRole } from "./use-delete-role.ts";
import { isUsed, roleUsedText } from "./used-by-text.ts";

export interface DeleteRoleProps {
  readonly id: string;
  readonly name: string;
  readonly used: Schema<"RoleUsedByBody">;
}

/**
 * *Delete the role*: a dialog that says exactly what happens (UX principle 7). A role that somebody
 * has is not deleted without the role that takes its place: everybody who has it gets that role in
 * the same place.
 */
export function DeleteRole({ id, name, used }: DeleteRoleProps) {
  const [replacement, setReplacement] = useState("");
  const remove = useDeleteRole();
  // The viewer may not see every place the role is used, so the server can still ask for one.
  const asks = isUsed(used) || codeOf(remove.error) === "role.in_use";
  return (
    <div className={styles.stack}>
      {remove.isError ? <Alert danger>{messageOf(remove.error)}</Alert> : null}
      <div className={styles.row}>
        <ConfirmDialog
          triggerLabel={t("roles.delete")}
          triggerName={t("roles.delete-name", { name })}
          title={t("roles.delete-title", { name })}
          text={asks ? `${roleUsedText(used)} ${t("roles.delete-used")}` : t("roles.delete-unused")}
          confirmLabel={t("roles.delete")}
          cancelLabel={t("common.cancel")}
          confirmDisabled={asks && replacement === ""}
          danger
          onClosed={() => {
            setReplacement("");
          }}
          onConfirm={() => {
            remove.mutate({
              params: {
                path: { role_id: id },
                query: replacement === "" ? {} : { replacement },
              },
            });
          }}
        >
          {asks ? <ReplacementField id={id} value={replacement} onChange={setReplacement} /> : null}
        </ConfirmDialog>
      </div>
    </div>
  );
}
