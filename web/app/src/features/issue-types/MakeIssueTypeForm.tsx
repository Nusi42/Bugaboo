import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Form,
  SubmitButton,
  TextField,
  partOf,
  part,
  type IssueTypeIconName,
} from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshIssueTypes } from "../../shared/query-keys.ts";
import { IconField } from "./IconField.tsx";

/** *Make an issue type*: a name and an icon; the type goes to the end of the list. */
export function MakeIssueTypeForm() {
  const [name, setName] = useState("");
  const [icon, setIcon] = useState<IssueTypeIconName>("star");
  const [made, setMade] = useState<string | null>(null);
  const make = api.query.useMutation("post", "/api/v1/issue-types", {
    onSuccess: (issueType) => {
      refreshIssueTypes();
      setName("");
      setMade(issueType.name ?? name);
    },
  });
  return (
    <Form
      onSubmit={() => {
        setMade(null);
        make.mutate({ body: { name: name.trim(), icon } });
      }}
    >
      <TextField
        label={t("issue-types.field-name")}
        requiredMessage={t("error.field.required")}
        value={name}
        onChange={setName}
      />
      <IconField value={icon} onChange={setIcon} />
      {make.isError ? <Alert tone={alertTone.danger}>{messageOf(make.error)}</Alert> : null}
      {made === null ? null : (
        <Alert tone={alertTone.success}>{t("issue-types.made", { name: made })}</Alert>
      )}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={make.isPending}>{t("issue-types.make")}</SubmitButton>
      </div>
    </Form>
  );
}
