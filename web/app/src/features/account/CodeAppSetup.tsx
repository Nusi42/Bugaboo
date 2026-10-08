import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  autoComplete,
  Form,
  SubmitButton,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import type { Schema } from "@bugaboo/api-client";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { ConfirmPasswordField } from "./ConfirmPasswordField.tsx";
import { refreshTwoStep } from "./two-step.ts";

type Setup = Schema<"CodeAppBody">;

/** The picture and the key for the code app, and the first code that proves it works. */
function Scan({ setup, password, onSwitchedOn }: ScanProps) {
  const [code, setCode] = useState("");
  const picture = `data:image/png;base64,${setup.qr_png_base64}`;
  const confirm = api.query.useMutation("post", "/api/v1/auth/two-step/code-app/confirm", {
    onSuccess: (answer: Schema<"BackupCodesBody">) => {
      refreshTwoStep();
      onSwitchedOn(answer.codes);
    },
  });
  return (
    <Form
      onSubmit={() => {
        confirm.mutate({ body: { code: code.trim(), password } });
      }}
    >
      <p {...partOf(part.text)}>{t("code-app.scan")}</p>
      <img {...partOf(part.accountQr)} alt={t("code-app.qr-alt")} src={picture} />
      <p {...partOf(part.muted)}>{t("code-app.key-label")}</p>
      <code {...partOf(part.accountKey)}>{setup.secret}</code>
      <TextField
        label={t("code-app.confirm-code")}
        description={t("code-app.confirm-help")}
        autoComplete={autoComplete.oneTimeCode}
        requiredMessage={t("validation.required")}
        value={code}
        onChange={setCode}
      />
      {confirm.isError ? <Alert tone={alertTone.danger}>{messageOf(confirm.error)}</Alert> : null}
      <SubmitButton isPending={confirm.isPending}>{t("code-app.confirm")}</SubmitButton>
    </Form>
  );
}

interface ScanProps {
  readonly setup: Setup;
  readonly password: string;
  readonly onSwitchedOn: (codes: readonly string[]) => void;
}

export interface CodeAppSetupProps {
  /** The code app is on: here are the backup codes that were made with it. */
  readonly onSwitchedOn: (codes: readonly string[]) => void;
}

/** *Set up the code app*: the password, then the picture, then the first code. */
export function CodeAppSetup({ onSwitchedOn }: CodeAppSetupProps) {
  const [password, setPassword] = useState("");
  const start = api.query.useMutation("post", "/api/v1/auth/two-step/code-app");
  if (start.data !== undefined) {
    return <Scan setup={start.data} password={password} onSwitchedOn={onSwitchedOn} />;
  }
  return (
    <Form
      onSubmit={() => {
        start.mutate({ body: { password } });
      }}
    >
      <ConfirmPasswordField value={password} onChange={setPassword} />
      {start.isError ? <Alert tone={alertTone.danger}>{messageOf(start.error)}</Alert> : null}
      <SubmitButton isPending={start.isPending}>{t("code-app.setup")}</SubmitButton>
    </Form>
  );
}
