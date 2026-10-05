import { t } from "@bugaboo/messages";
import { Alert } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import styles from "./Account.module.css";
import { RemoveWithPassword } from "./RemoveWithPassword.tsx";
import { refreshTwoStep } from "./two-step.ts";

export interface SwitchOffProps {
  /** The code app was switched off. */
  readonly onSwitchedOff: () => void;
}

/** The code app is on: say so, and offer to switch it off. */
export function SwitchOff({ onSwitchedOff }: SwitchOffProps) {
  const remove = api.query.useMutation("post", "/api/v1/auth/two-step/code-app/remove", {
    onSuccess: () => {
      refreshTwoStep();
      onSwitchedOff();
    },
  });
  return (
    <>
      <p className={styles.text}>{t("code-app.on")}</p>
      {remove.isError ? <Alert danger>{messageOf(remove.error)}</Alert> : null}
      <div className={styles.row}>
        <RemoveWithPassword
          triggerLabel={t("code-app.turn-off")}
          title={t("code-app.turn-off-title")}
          text={t("code-app.turn-off-text")}
          confirmLabel={t("code-app.turn-off")}
          cancelLabel={t("common.cancel")}
          onConfirm={(password) => {
            remove.mutate({ body: { password } });
          }}
        />
      </div>
    </>
  );
}
