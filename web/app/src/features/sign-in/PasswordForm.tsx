import { t } from "@bugaboo/messages";
import { Alert, autoComplete, Button, Form, TextField } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { useAfterSignIn } from "../../shared/after-sign-in.ts";
import { messageOf } from "../../shared/errors.ts";

/** The e-mail address and the password of *Sign in*. */
export function PasswordForm() {
  const afterSignIn = useAfterSignIn();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const signIn = api.query.useMutation("post", "/api/v1/auth/sign-in", { onSuccess: afterSignIn });
  return (
    <Form
      onSubmit={() => {
        signIn.mutate({ body: { email: email.trim(), password } });
      }}
    >
      <TextField
        label={t("field.email")}
        email
        autoComplete={autoComplete.username}
        requiredMessage={t("validation.required")}
        emailMessage={t("validation.email")}
        value={email}
        onChange={setEmail}
      />
      <TextField
        label={t("field.password")}
        password
        autoComplete={autoComplete.currentPassword}
        requiredMessage={t("validation.required")}
        value={password}
        onChange={setPassword}
      />
      {signIn.isError ? <Alert danger>{messageOf(signIn.error)}</Alert> : null}
      <Button primary wide submit isPending={signIn.isPending}>
        {t("sign-in.submit")}
      </Button>
    </Form>
  );
}
