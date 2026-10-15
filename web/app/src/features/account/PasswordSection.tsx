import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  autoComplete,
  Card,
  fieldKind,
  Form,
  SubmitButton,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { isPasswordProblem, messageOf } from "../../shared/errors.ts";

/** What came of the last try: a problem that is not a weak password, or the good news. */
function Outcome({ problem, done }: { readonly problem: string | null; readonly done: boolean }) {
  if (problem !== null) {
    return <Alert tone={alertTone.danger}>{problem}</Alert>;
  }
  return done ? <Alert tone={alertTone.success}>{t("security.password.changed")}</Alert> : null;
}

/** *Password*: change it, after typing the one that is used now. */
export function PasswordSection() {
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const change = api.query.useMutation("post", "/api/v1/auth/password", {
    onSuccess: () => {
      setCurrent("");
      setNext("");
    },
  });
  const tooWeak = isPasswordProblem(change.error);
  return (
    <Card heading={t("security.password.title")}>
      <Form
        onSubmit={() => {
          change.mutate({ body: { current_password: current, new_password: next } });
        }}
      >
        <TextField
          label={t("security.password.current")}
          kind={fieldKind.password}
          autoComplete={autoComplete.currentPassword}
          requiredMessage={t("error.field.required")}
          value={current}
          onChange={setCurrent}
        />
        <TextField
          label={t("security.password.new")}
          description={t("field.password-help")}
          kind={fieldKind.password}
          autoComplete={autoComplete.newPassword}
          requiredMessage={t("error.field.required")}
          errorMessage={tooWeak ? messageOf(change.error) : undefined}
          value={next}
          onChange={(value) => {
            change.reset();
            setNext(value);
          }}
        />
        <Outcome
          problem={change.isError && !tooWeak ? messageOf(change.error) : null}
          done={change.isSuccess}
        />
        <div {...partOf(part.row, { variants: ["stretch"] })}>
          <SubmitButton isPending={change.isPending}>{t("security.password.submit")}</SubmitButton>
        </div>
      </Form>
    </Card>
  );
}
