import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { passkeysSupported } from "../../shared/passkey.ts";
import { statusOptions } from "../../shared/session.ts";
import styles from "./Account.module.css";
import { AddPasskeyForm } from "./AddPasskeyForm.tsx";
import { PasskeyList } from "./PasskeyList.tsx";
import { twoStepOptions } from "./two-step.ts";

/** *Passkeys*: the ones the person has, and a way to add one. */
export function PasskeysSection() {
  const available = useQuery(statusOptions()).data?.passkeys_available;
  const passkeys = useQuery(twoStepOptions()).data?.passkeys ?? [];
  const [removed, setRemoved] = useState(false);
  return (
    <Card heading={t("passkeys.title")}>
      <p className={styles.text}>{t("passkeys.intro")}</p>
      {available === false ? (
        <Alert tone={alertTone.warning}>{t("passkeys.unavailable")}</Alert>
      ) : null}
      {available === true && !passkeysSupported() ? (
        <Alert tone={alertTone.warning}>{t("passkeys.not-supported")}</Alert>
      ) : null}
      {available === true && passkeysSupported() ? <AddPasskeyForm /> : null}
      {removed ? <Alert tone={alertTone.success}>{t("passkeys.removed")}</Alert> : null}
      <PasskeyList
        passkeys={passkeys}
        onRemoved={() => {
          setRemoved(true);
        }}
      />
    </Card>
  );
}
