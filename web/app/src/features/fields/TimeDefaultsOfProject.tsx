import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, NameValueList } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshFields } from "../../shared/query-keys.ts";
import { useValuesFromAbove } from "./defaults-from-above.ts";
import { useFields, type FieldValues } from "./fields.ts";
import { lineKey } from "./line-key.ts";
import { TimeDefaultLine } from "./TimeDefaultLine.tsx";

type Project = Schema<"ProjectDetailBody">;

/**
 * *Activity*, *Cost center* and the other fields of time entries as defaults of one project (TIME-13,
 * ADR-0035, decision 1): every entry booked on an issue of the project starts with them, unless the
 * issue sets its own; sub-projects inherit them (*Set by the project above*) until they set their
 * own (*Reset to default* takes it away). The lists themselves are managed on *Fields*.
 */
export function TimeDefaultsOfProject({ project }: { readonly project: Project }) {
  const shortName = project.project.short_name;
  const fields = useFields({ project: shortName });
  const above = useValuesFromAbove(
    project.trail.filter((each) => each.id !== project.project.id).map((each) => each.short_name),
  );
  const change = api.query.useMutation("patch", "/api/v1/projects/{project}", {
    onSuccess: refreshFields,
  });
  const changeable = new Set<string>(project.changeable_fields);
  const shown = (fields.data?.fields ?? []).filter(
    (field) => field.applies_to === "time-entries" && changeable.has(field.id),
  );
  if (shown.length === 0) {
    return null;
  }
  const values: FieldValues = project.own_fields;
  return (
    <Card
      heading={t("fields.time-defaults-project")}
      intro={t("fields.time-defaults-project-intro")}
    >
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
      <NameValueList>
        {shown.map((field) => (
          <TimeDefaultLine
            key={lineKey(field, values[field.id], project.project.version)}
            field={field}
            project={shortName}
            own={values[field.id]}
            above={above[field.id]}
            onCommit={(each, value) => {
              change.mutate({
                params: { path: { project: project.project.id } },
                body: { version: project.project.version, fields: { [each.id]: value } },
              });
            }}
          />
        ))}
      </NameValueList>
    </Card>
  );
}
