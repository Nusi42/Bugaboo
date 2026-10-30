import { handleChangeEveryDays, handleMax } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  autoComplete,
  Card,
  Form,
  SubmitButton,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { codeOf, messageOf } from "../../shared/errors.ts";
import { queryClient } from "../../shared/query-client.ts";
import { usePageTitle } from "../../shared/use-page.ts";

/** The words for what the server said about a handle: taken, not a handle, or changed too lately. */
function problemWords(error: unknown): string {
  const code = codeOf(error);
  if (code === "name.taken") {
    return t("handle.taken");
  }
  return code === "request.invalid" ? t("handle.invalid") : messageOf(error);
}

/** The rules of a handle, in plain words. The numbers are the server's. */
function Rules() {
  return (
    <section {...partOf(part.formSection)} aria-label={t("handle.rules")}>
      <h2 {...partOf(part.formSectionTitle)}>{t("handle.rules")}</h2>
      <ul {...partOf(part.list)}>
        <li>{t("handle.rule-characters", { max: handleMax })}</li>
        <li>{t("handle.rule-own")}</li>
        <li>{t("handle.rule-weekly", { days: handleChangeEveryDays })}</li>
        <li>{t("handle.rule-kept")}</li>
      </ul>
    </section>
  );
}

/** The box with the handle the person has, and the button that changes it. */
function HandleForm({ handle }: { readonly handle: string }) {
  const [value, setValue] = useState(handle);
  const change = api.query.useMutation("put", "/api/v1/me/handle", {
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: api.query.queryOptions("get", "/api/v1/me/handle").queryKey,
      });
    },
  });
  return (
    <Form
      onSubmit={() => {
        change.mutate({ body: { handle: value.trim() } });
      }}
    >
      <TextField
        label={t("handle.label")}
        autoComplete={autoComplete.off}
        requiredMessage={t("error.field.required")}
        value={value}
        onChange={setValue}
      />
      {change.isError ? <Alert tone={alertTone.danger}>{problemWords(change.error)}</Alert> : null}
      {change.isSuccess ? (
        <Alert tone={alertTone.success}>
          {t("handle.changed", { handle: change.data.handle })}
        </Alert>
      ) : null}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={change.isPending}>{t("handle.save")}</SubmitButton>
      </div>
    </Form>
  );
}

/**
 * *My settings › Handle* (M3-F70, ADR-0033 §6): the short name people type after the @ to mention
 * the person, and the rules for changing it.
 */
export function HandlePage() {
  usePageTitle(t("handle.title"));
  const current = api.query.useQuery("get", "/api/v1/me/handle");
  return (
    <Card heading={t("handle.title")} level={1} intro={t("handle.intro")}>
      {current.isError ? <Alert tone={alertTone.danger}>{messageOf(current.error)}</Alert> : null}
      {current.data === undefined ? null : <HandleForm handle={current.data.handle} />}
      <Rules />
    </Card>
  );
}
