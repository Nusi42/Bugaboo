import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, SubmitButton, TextField } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { ConfirmPasswordField } from "./ConfirmPasswordField.tsx";
import { useAddPasskey } from "./use-add-passkey.ts";

/** *Add a passkey*: a name for it, and the password typed again. */
export function AddPasskeyForm() {
  const [label, setLabel] = useState("");
  const [password, setPassword] = useState("");
  const { add, busy, outcome } = useAddPasskey();
  return (
    <Form
      onSubmit={() => {
        void add(label.trim(), password).then(() => {
          setPassword("");
        });
      }}
    >
      <TextField
        label={t("passkeys.name")}
        description={t("passkeys.name-help")}
        requiredMessage={t("error.field.required")}
        value={label}
        onChange={setLabel}
      />
      <ConfirmPasswordField value={password} onChange={setPassword} />
      {outcome?.kind === "added" ? (
        <Alert tone={alertTone.success}>{t("passkeys.added", { name: outcome.label })}</Alert>
      ) : null}
      {outcome?.kind === "cancelled" ? <Alert>{t("passkeys.cancelled")}</Alert> : null}
      {outcome?.kind === "failed" ? (
        <Alert tone={alertTone.danger}>{messageOf(outcome.error)}</Alert>
      ) : null}
      <SubmitButton isPending={busy}>{t("passkeys.add")}</SubmitButton>
    </Form>
  );
}
