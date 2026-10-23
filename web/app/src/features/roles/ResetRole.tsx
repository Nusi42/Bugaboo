import { t } from "@bugaboo/messages";
import { Alert, alertTone, ConfirmDialog, partOf, part } from "@bugaboo/ui";

import { refreshAccess } from "../../shared/query-keys.ts";
import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";

export interface ResetRoleProps {
  readonly id: string;
  readonly name: string;
  /** The version of the role that was looked at. */
  readonly version: number;
}

/**
 * *Reset to the starting permissions* for a starting role: the permissions and the switch the role
 * had at the start come back; the name stays. A dialog says exactly what happens.
 */
export function ResetRole({ id, name, version }: ResetRoleProps) {
  const reset = api.query.useMutation("post", "/api/v1/roles/{role}/reset", {
    onSuccess: refreshAccess,
  });
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.muted)}>{t("roles.reset-help")}</p>
      {reset.isError ? <Alert tone={alertTone.danger}>{messageOf(reset.error)}</Alert> : null}
      {reset.isSuccess ? (
        <Alert tone={alertTone.success}>{t("roles.reset-done", { name })}</Alert>
      ) : null}
      <div {...partOf(part.row)}>
        <ConfirmDialog
          triggerLabel={t("roles.reset")}
          title={t("roles.reset-title", { name })}
          text={t("roles.reset-text", { name })}
          confirmLabel={t("roles.reset")}
          cancelLabel={t("common.cancel")}
          onConfirm={() => {
            reset.mutate({ params: { path: { role: id } }, body: { version } });
          }}
        />
      </div>
    </div>
  );
}
