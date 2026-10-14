import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshFields } from "../../shared/query-keys.ts";
import { useFields, type FieldValues } from "./fields.ts";
import { FieldLine } from "./FieldLine.tsx";
import { lineKey } from "./line-key.ts";

/**
 * The own fields of a project on its overview (CF-08): the same fields with the scope *Projects*,
 * in their look, and an editor for the person who may change the project.
 */
export function OwnFieldsOfProject({ project }: { readonly project: Schema<"ProjectDetailBody"> }) {
  const shortName = project.project.short_name;
  const fields = useFields({ project: shortName });
  const change = api.query.useMutation("patch", "/api/v1/projects/{project}", {
    onSuccess: refreshFields,
  });
  const shown = (fields.data?.fields ?? []).filter((field) => field.applies_to === "projects");
  const values: FieldValues = project.own_fields;
  const changeable = new Set<string>(project.changeable_fields);
  if (shown.length === 0) {
    return null;
  }
  return (
    <Card heading={t("fields.project-heading")}>
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
      {shown.map((field) => (
        <FieldLine
          key={lineKey(field, values[field.id], project.project.version)}
          field={field}
          project={shortName}
          value={values[field.id]}
          editable={changeable.has(field.id)}
          onCommit={(each, value) => {
            change.mutate({
              params: { path: { project: project.project.id } },
              body: { version: project.project.version, fields: { [each.id]: value } },
            });
          }}
        />
      ))}
    </Card>
  );
}
