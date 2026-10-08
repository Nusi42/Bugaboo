import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, Form, SubmitButton, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { type SignedIn } from "../../shared/after-sign-in.ts";
import { api } from "../../shared/api.ts";
import { codeOf, isPasswordProblem, messageOf } from "../../shared/errors.ts";
import { EmailField, NameField, NewPasswordField } from "../../shared/fields.tsx";
import { queryClient } from "../../shared/query-client.ts";
import { forgetPerson, statusOptions } from "../../shared/session.ts";

export interface AccountStepProps {
  /** The setup code from the first step. */
  readonly code: string;
  /** The account is made and the person is signed in. */
  readonly onCreated: () => void;
  /** The server did not accept the setup code: back to the first step, with its message. */
  readonly onWrongCode: (message: string) => void;
}

/** The second step: the name, e-mail address and password of the first administrator. */
export function AccountStep({ code, onCreated, onWrongCode }: AccountStepProps) {
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const setup = api.query.useMutation("post", "/api/v1/auth/setup", {
    onSuccess: (answer: SignedIn) => {
      if (answer.person !== null && answer.person !== undefined) {
        forgetPerson();
      }
      void queryClient.invalidateQueries({ queryKey: statusOptions().queryKey });
      onCreated();
    },
    onError: (error: unknown) => {
      if (codeOf(error) === "auth.wrong_setup_code") {
        onWrongCode(messageOf(error));
      }
    },
  });
  const tooWeak = isPasswordProblem(setup.error);
  return (
    <Card heading={t("first-run.account.title")} level={1}>
      <p {...partOf(part.text)}>{t("first-run.account.intro")}</p>
      <Form
        onSubmit={() => {
          setup.mutate({
            body: { code: code.trim(), display_name: name.trim(), email: email.trim(), password },
          });
        }}
      >
        <NameField value={name} onChange={setName} />
        <EmailField value={email} onChange={setEmail} />
        <NewPasswordField
          value={password}
          onChange={(value) => {
            setup.reset();
            setPassword(value);
          }}
          problem={tooWeak ? messageOf(setup.error) : undefined}
        />
        {setup.isError && !tooWeak ? (
          <Alert tone={alertTone.danger}>{messageOf(setup.error)}</Alert>
        ) : null}
        <SubmitButton wide isPending={setup.isPending}>
          {t("first-run.account.submit")}
        </SubmitButton>
      </Form>
    </Card>
  );
}
