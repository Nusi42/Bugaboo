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

export interface CreateIssueTypeFormProps {
  /** The project the type is created in (a short name); the whole organization when left out. */
  readonly project?: string;
}

/**
 * *Create an issue type*: a name and an icon; the type goes to the end of the list. For a project
 * it is offered there and below.
 */
export function CreateIssueTypeForm({ project }: CreateIssueTypeFormProps) {
  const [name, setName] = useState("");
  const [icon, setIcon] = useState<IssueTypeIconName>("star");
  const [made, setMade] = useState<string | null>(null);
  const create = api.query.useMutation("post", "/api/v1/issue-types", {
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
        create.mutate({
          body: { name: name.trim(), icon, ...(project === undefined ? {} : { project }) },
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
      {create.isError ? <Alert tone={alertTone.danger}>{messageOf(create.error)}</Alert> : null}
      {made === null ? null : (
        <Alert tone={alertTone.success}>{t("issue-types.created", { name: made })}</Alert>
      )}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={create.isPending}>{t("issue-types.create")}</SubmitButton>
      </div>
    </Form>
  );
}
