import { t } from "@bugaboo/messages";
import { Alert, Button, Form } from "@bugaboo/ui";
import { useState } from "react";

import { useAfterSignIn } from "../../shared/after-sign-in.ts";
import { api } from "../../shared/api.ts";
import { isPasswordProblem, messageOf } from "../../shared/errors.ts";
import { NameField, NewPasswordField } from "../../shared/fields.tsx";

export interface AcceptFormProps {
  /** The secret from the invitation link. */
  readonly secret: string;
  /** The address the account will get. */
  readonly email: string;
}

/** The name and password of somebody who is invited: the e-mail address comes with the invitation. */
export function AcceptForm({ secret, email }: AcceptFormProps) {
  const afterSignIn = useAfterSignIn();
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const accept = api.query.useMutation("post", "/api/v1/auth/invitation/accept", {
    onSuccess: (answer) => {
      // The secret is used up: it leaves the address and the history of the browser.
      window.history.replaceState(null, "", window.location.pathname + window.location.search);
      afterSignIn(answer);
    },
  });
  const tooWeak = isPasswordProblem(accept.error);
  return (
    <Form
      onSubmit={() => {
        accept.mutate({ body: { secret, display_name: name.trim(), password } });
      }}
    >
      <p>{t("invitation.intro", { email })}</p>
      <NameField value={name} onChange={setName} />
      <NewPasswordField
        value={password}
        onChange={(value) => {
          accept.reset();
          setPassword(value);
        }}
        problem={tooWeak ? messageOf(accept.error) : undefined}
      />
      {accept.isError && !tooWeak ? <Alert danger>{messageOf(accept.error)}</Alert> : null}
      <Button primary wide submit isPending={accept.isPending}>
        {t("invitation.submit")}
      </Button>
    </Form>
  );
}
