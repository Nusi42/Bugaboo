import { t } from "@bugaboo/messages";
import { Alert, Button, Form } from "@bugaboo/ui";
import { useState } from "react";

import { useAfterSignIn } from "../../shared/after-sign-in.ts";
import { api } from "../../shared/api.ts";
import { isPasswordProblem, messageOf } from "../../shared/errors.ts";
import { EmailField, NameField, NewPasswordField } from "../../shared/fields.tsx";

/** The form of the sign-up page, for the days when an administrator allows signing up. */
export function SignUpForm() {
  const afterSignIn = useAfterSignIn();
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const signUp = api.query.useMutation("post", "/api/v1/auth/sign-up", { onSuccess: afterSignIn });
  const tooWeak = isPasswordProblem(signUp.error);
  return (
    <Form
      onSubmit={() => {
        signUp.mutate({ body: { display_name: name.trim(), email: email.trim(), password } });
      }}
    >
      <NameField value={name} onChange={setName} />
      <EmailField value={email} onChange={setEmail} />
      <NewPasswordField
        value={password}
        onChange={(value) => {
          signUp.reset();
          setPassword(value);
        }}
        problem={tooWeak ? messageOf(signUp.error) : undefined}
      />
      {signUp.isError && !tooWeak ? <Alert danger>{messageOf(signUp.error)}</Alert> : null}
      <Button primary wide submit isPending={signUp.isPending}>
        {t("sign-up.submit")}
      </Button>
    </Form>
  );
}
