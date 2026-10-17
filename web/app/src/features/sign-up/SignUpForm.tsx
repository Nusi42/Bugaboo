import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, SubmitButton } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { EmailField } from "../../shared/fields.tsx";

/**
 * The form of the sign-up page, for the days when an administrator allows signing up. The person
 * gives only an address; the link that makes the account goes there, and the screens that follow
 * it are the ones of an invitation. The answer is the same for every address, so the page says
 * the same too.
 */
export function SignUpForm() {
  const [email, setEmail] = useState("");
  const signUp = api.query.useMutation("post", "/api/v1/auth/sign-up");
  if (signUp.isSuccess) {
    return <Alert tone={alertTone.success}>{t("sign-up.sent")}</Alert>;
  }
  return (
    <Form
      onSubmit={() => {
        signUp.mutate({ body: { email: email.trim() } });
      }}
    >
      <EmailField value={email} onChange={setEmail} />
      {signUp.isError ? <Alert tone={alertTone.danger}>{messageOf(signUp.error)}</Alert> : null}
      <SubmitButton wide isPending={signUp.isPending}>
        {t("sign-up.submit")}
      </SubmitButton>
    </Form>
  );
}
