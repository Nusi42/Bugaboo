import { t } from "@bugaboo/messages";
import { Alert, alertTone, autoComplete, Form, SubmitButton, TextField } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { useAfterSignIn } from "../../shared/after-sign-in.ts";
import { codeOf, messageOf } from "../../shared/errors.ts";

/** The code of the second step: from the code app, or one of the backup codes. */
export function CodeForm() {
  const afterSignIn = useAfterSignIn();
  const [code, setCode] = useState("");
  const finish = api.query.useMutation("post", "/api/v1/auth/second-step", {
    onSuccess: afterSignIn,
  });
  const wrongCode = codeOf(finish.error) === "auth.wrong-second-step-code";
  return (
    <Form
      onSubmit={() => {
        finish.mutate({ body: { code: code.trim() } });
      }}
    >
      <TextField
        label={t("second-step.code")}
        description={t("second-step.code-help")}
        autoComplete={autoComplete.oneTimeCode}
        requiredMessage={t("error.field.required")}
        errorMessage={wrongCode ? messageOf(finish.error) : undefined}
        value={code}
        onChange={(value) => {
          // A problem the server named is about the value that was sent, not the one being typed.
          finish.reset();
          setCode(value);
        }}
      />
      {finish.isError && !wrongCode ? (
        <Alert tone={alertTone.danger}>{messageOf(finish.error)}</Alert>
      ) : null}
      <SubmitButton wide isPending={finish.isPending}>
        {t("second-step.submit")}
      </SubmitButton>
    </Form>
  );
}
