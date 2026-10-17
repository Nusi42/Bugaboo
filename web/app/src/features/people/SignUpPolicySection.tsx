import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  buttonLook,
  Card,
  ConfirmDialog,
  fieldKind,
  Form,
  SubmitButton,
  Switch,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { queryClient } from "../../shared/query-client.ts";
import { statusOptions } from "../../shared/session.ts";
import { MailOffNote } from "./MailOffNote.tsx";

/** The rule for signing up, as the server has it now. */
function policyOptions() {
  return api.query.queryOptions("get", "/api/v1/auth/sign-up-policy");
}

/** The domains as the administrator typed them: one per line, or separated by commas or spaces. */
function domainsOf(text: string): string[] {
  return text
    .split(/[\s,]+/u)
    .map((domain) => domain.trim())
    .filter((domain) => domain !== "");
}

/** The switch and the list of domains, with the warning that belongs to an empty list (M1-F46). */
function PolicyForm({ policy }: { readonly policy: Schema<"SignUpPolicyBody"> }) {
  const [enabled, setEnabled] = useState(policy.enabled);
  const [text, setText] = useState(policy.domains.join("\n"));
  const save = api.query.useMutation("put", "/api/v1/auth/sign-up-policy", {
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: statusOptions().queryKey });
    },
    // Also after a refusal (a stale version): the policy is read again, so the next save is made
    // on the version the server has now and the form is not stuck on the old one.
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: policyOptions().queryKey });
    },
  });
  const domains = domainsOf(text);
  const everybody = enabled && domains.length === 0;
  const send = (): void => {
    save.mutate({ body: { enabled, domains, version: policy.version } });
  };
  return (
    <Form onSubmit={send}>
      <Switch label={t("sign-up-policy.switch")} isSelected={enabled} onChange={setEnabled} />
      <TextField
        label={t("sign-up-policy.domains")}
        description={t("sign-up-policy.domains-help")}
        kind={fieldKind.multiline}
        value={text}
        onChange={setText}
      />
      <MailOffNote enabled={policy.enabled} />
      {everybody ? <Alert tone={alertTone.warning}>{t("sign-up-policy.warning")}</Alert> : null}
      {save.isError ? <Alert tone={alertTone.danger}>{messageOf(save.error)}</Alert> : null}
      {save.isSuccess ? <Alert tone={alertTone.success}>{t("save.saved")}</Alert> : null}
      <div {...partOf(part.row)}>
        {everybody ? (
          <ConfirmDialog
            triggerLabel={t("common.save")}
            title={t("sign-up-policy.confirm-title")}
            text={t("sign-up-policy.confirm-text")}
            confirmLabel={t("sign-up-policy.confirm-yes")}
            cancelLabel={t("sign-up-policy.confirm-no")}
            confirmLook={buttonLook.danger}
            onConfirm={send}
          />
        ) : (
          <SubmitButton isPending={save.isPending}>{t("common.save")}</SubmitButton>
        )}
      </div>
    </Form>
  );
}

/** *Signing up without an invitation*: who may make their own account. */
export function SignUpPolicySection() {
  const policy = useQuery(policyOptions());
  return (
    <Card heading={t("sign-up-policy.title")}>
      <p {...partOf(part.text)}>{t("sign-up-policy.intro")}</p>
      {policy.isError ? <Alert tone={alertTone.danger}>{messageOf(policy.error)}</Alert> : null}
      {policy.data === undefined ? null : <PolicyForm policy={policy.data} />}
    </Card>
  );
}
