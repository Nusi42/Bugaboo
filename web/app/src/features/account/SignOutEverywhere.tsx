import { t } from "@bugaboo/messages";
import { Alert, alertTone, buttonLook, ConfirmDialog } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { paths } from "../../shared/paths.ts";
import { leaveSession } from "../../shared/session.ts";
import styles from "./Account.module.css";

/** *Sign out everywhere*: every session of the person ends, this one included. */
export function SignOutEverywhere() {
  const navigate = useNavigate();
  const everywhere = api.query.useMutation("delete", "/api/v1/auth/sessions", {
    onSuccess: () =>
      leaveSession(() => navigate({ to: paths.signIn, search: { reason: "signed-out" } })),
  });
  return (
    <>
      {everywhere.isError ? (
        <Alert tone={alertTone.danger}>{messageOf(everywhere.error)}</Alert>
      ) : null}
      <div className={styles.row}>
        <ConfirmDialog
          triggerLabel={t("sessions.everywhere")}
          title={t("sessions.everywhere-title")}
          text={t("sessions.everywhere-text")}
          confirmLabel={t("sessions.everywhere")}
          cancelLabel={t("common.cancel")}
          confirmLook={buttonLook.danger}
          onConfirm={() => {
            everywhere.mutate({});
          }}
        />
      </div>
    </>
  );
}
