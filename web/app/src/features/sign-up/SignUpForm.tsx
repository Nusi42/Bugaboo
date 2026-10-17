import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, SubmitButton } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { EmailField } from "../../shared/fields.tsx";

export interface SignUpFormProps {
  /** Called with the address once the server took it: the page then says to look at the mail. */
  readonly onSent: (email: string) => void;
}

/**
 * The form of the sign-up page, for the days when an administrator allows signing up. The person
 * gives only an address; the link that makes the account goes there, and the screens that follow it
 * are the ones of an invitation. The answer is the same for every address.
 */
export function SignUpForm({ onSent }: SignUpFormProps) {
  const [email, setEmail] = useState("");
  const signUp = api.query.useMutation("post", "/api/v1/auth/sign-up");
  return (
    <Form
      onSubmit={() => {
        const address = email.trim();
        signUp.mutate(
          { body: { email: address } },
          {
            onSuccess: () => {
              onSent(address);
            },
          },
        );
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
