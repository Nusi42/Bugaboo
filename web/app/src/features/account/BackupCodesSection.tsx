import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonType, Card, Form, partOf, part } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { BackupCodes } from "./BackupCodes.tsx";
import { ConfirmPasswordField } from "./ConfirmPasswordField.tsx";
import { refreshTwoStep, twoStepOptions } from "./two-step.ts";

/** *Backup codes*: how many are left, and a way to make new ones. Only with a code app. */
export function BackupCodesSection() {
  const twoStep = useQuery(twoStepOptions()).data;
  const [password, setPassword] = useState("");
  const renew = api.query.useMutation("post", "/api/v1/auth/two-step/backup-codes", {
    onSuccess: () => {
      setPassword("");
      refreshTwoStep();
    },
  });
  if (twoStep?.code_app !== true) {
    return null;
  }
  return (
    <Card heading={t("backup.title")}>
      <p {...partOf(part.text)}>{t("backup.intro")}</p>
      <p {...partOf(part.text)}>{t("backup.left", { count: twoStep.backup_codes_left })}</p>
      {renew.data === undefined ? null : (
        <BackupCodes codes={renew.data.codes} onSaved={renew.reset} />
      )}
      <Form
        onSubmit={() => {
          renew.mutate({ body: { password } });
        }}
      >
        <ConfirmPasswordField value={password} onChange={setPassword} />
        <p {...partOf(part.muted)}>{t("backup.renew-help")}</p>
        {renew.isError ? <Alert tone={alertTone.danger}>{messageOf(renew.error)}</Alert> : null}
        <div {...partOf(part.row)}>
          <Button type={buttonType.submit} isPending={renew.isPending}>
            {t("backup.renew")}
          </Button>
        </div>
      </Form>
    </Card>
  );
}
