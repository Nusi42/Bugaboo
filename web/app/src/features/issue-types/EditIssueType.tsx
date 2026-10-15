import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, SubmitButton, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshIssueTypes } from "../../shared/query-keys.ts";
import { IconField } from "./IconField.tsx";
import type { IssueTypeLine } from "./issue-types.ts";

/** *Name* and *Icon* of the issue type, saved with the button on the version that was looked at. */
export function EditIssueType({ issueType }: { readonly issueType: IssueTypeLine }) {
  const [name, setName] = useState(issueType.name);
  const [icon, setIcon] = useState(issueType.icon);
  const save = api.query.useMutation("patch", "/api/v1/issue-types/{issue_type}", {
    onSuccess: refreshIssueTypes,
  });
  return (
    <Form
      onSubmit={() => {
        save.mutate({
          params: { path: { issue_type: issueType.id } },
          body: { version: issueType.version, name: name.trim(), icon },
        });
      }}
    >
      <TextField
        label={t("issue-types.field-name")}
        requiredMessage={t("error.field.required")}
        value={name}
        onChange={setName}
      />
      <IconField value={icon} onChange={setIcon} />
      {save.isError ? <Alert tone={alertTone.danger}>{messageOf(save.error)}</Alert> : null}
      {save.isSuccess ? <Alert tone={alertTone.success}>{t("issue-types.saved")}</Alert> : null}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={save.isPending}>{t("common.save")}</SubmitButton>
      </div>
    </Form>
  );
}
