import { t } from "@bugaboo/messages";
import type { Schema } from "@bugaboo/api-client";
import { Alert, alertTone, Card, Form, SubmitButton, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshAccess } from "../../shared/query-keys.ts";
import { paths } from "../../shared/paths.ts";
import type { GroupLine } from "./groups.ts";
import { groupUsedText } from "./used-by-text.ts";

/** *Rename*: the name of the group, saved with the button. */
export function RenameGroup({ group }: { readonly group: GroupLine }) {
  const [name, setName] = useState(group.name);
  const rename = api.query.useMutation("patch", "/api/v1/groups/{group}", {
    onSuccess: refreshAccess,
  });
  return (
    <Form
      onSubmit={() => {
        rename.mutate({
          params: { path: { group: group.id } },
          body: { name: name.trim(), version: group.version },
        });
      }}
    >
      <TextField
        label={t("groups.field-name")}
        requiredMessage={t("error.field.required")}
        value={name}
        onChange={setName}
      />
      {rename.isError ? <Alert tone={alertTone.danger}>{messageOf(rename.error)}</Alert> : null}
      {rename.isSuccess ? <Alert tone={alertTone.success}>{t("save.saved")}</Alert> : null}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={rename.isPending}>{t("common.save")}</SubmitButton>
      </div>
    </Form>
  );
}

/** A new form for each name the group has, so the field shows the name that was saved. */
function renameKey(group: GroupLine): string {
  return `${group.id}:${group.name}`;
}

/** The first card of the page of a group: its name as the title, what it is used for, *Rename*, and *All groups*. */
export function GroupNameCard({
  group,
  used,
}: {
  readonly group: GroupLine;
  readonly used: Schema<"GroupUsedByBody">;
}) {
  return (
    <Card heading={group.name} level={1} back={{ href: paths.groups, label: t("groups.back") }}>
      <p {...partOf(part.muted)}>{groupUsedText(used)}</p>
      <RenameGroup key={renameKey(group)} group={group} />
    </Card>
  );
}
