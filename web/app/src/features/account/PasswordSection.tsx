import { t } from "@bugaboo/messages";
import { Alert, autoComplete, Button, Card, Form, TextField } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { isPasswordProblem, messageOf } from "../../shared/errors.ts";
import styles from "./Account.module.css";

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
          password
          autoComplete={autoComplete.currentPassword}
          requiredMessage={t("validation.required")}
          value={current}
          onChange={setCurrent}
        />
        <TextField
          label={t("security.password.new")}
          description={t("field.password-help")}
          password
          autoComplete={autoComplete.newPassword}
          requiredMessage={t("validation.required")}
          errorMessage={tooWeak ? messageOf(change.error) : undefined}
          value={next}
          onChange={(value) => {
            change.reset();
            setNext(value);
          }}
        />
        {change.isError && !tooWeak ? <Alert danger>{messageOf(change.error)}</Alert> : null}
        {change.isSuccess ? <Alert success>{t("security.password.changed")}</Alert> : null}
        <div className={styles.row}>
          <Button primary submit isPending={change.isPending}>
            {t("security.password.submit")}
          </Button>
        </div>
      </Form>
    </Card>
  );
}
