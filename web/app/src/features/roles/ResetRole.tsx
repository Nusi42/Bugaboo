import { t } from "@bugaboo/messages";
import { Alert, ConfirmDialog } from "@bugaboo/ui";

import { refreshAccess } from "../../shared/access-cache.ts";
import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import styles from "./Roles.module.css";

export interface ResetRoleProps {
  readonly id: string;
  readonly name: string;
}

/**
 * *Reset to the starting permissions* for a starting role: the permissions and the switch the role
 * had at the start come back; the name stays. A dialog says exactly what happens.
 */
export function ResetRole({ id, name }: ResetRoleProps) {
  const reset = api.query.useMutation("post", "/api/v1/roles/{role_id}/reset", {
    onSuccess: refreshAccess,
  });
  return (
    <div className={styles.stack}>
      <p className={styles.muted}>{t("roles.reset-help")}</p>
      {reset.isError ? <Alert danger>{messageOf(reset.error)}</Alert> : null}
      {reset.isSuccess ? <Alert success>{t("roles.reset-done", { name })}</Alert> : null}
      <div className={styles.row}>
        <ConfirmDialog
          triggerLabel={t("roles.reset")}
          title={t("roles.reset-title", { name })}
          text={t("roles.reset-text", { name })}
          confirmLabel={t("roles.reset")}
          cancelLabel={t("common.cancel")}
          onConfirm={() => {
            reset.mutate({ params: { path: { role_id: id } } });
          }}
        />
      </div>
    </div>
  );
}
