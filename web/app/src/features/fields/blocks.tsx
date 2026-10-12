import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, partOf, part } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshFields } from "../../shared/query-keys.ts";
import type { IssueBlockProps } from "../issues/index.ts";
import { issueFieldsOf, useFields, type OwnField } from "./fields.ts";
import { ValueEditor, type Sent } from "./ValueEditor.tsx";
import { ValueView } from "./ValueView.tsx";

/** The values of the own fields of an item, by the key of the field, as the server sent them. */
type Values = Readonly<Record<string, unknown>>;

/** What makes a line start afresh: the field, the value it holds and the version of the item. */
function lineKey(field: OwnField, value: unknown, version = 0): string {
  return [field.id, JSON.stringify(value), version].join("|");
}

interface FieldLineProps {
  readonly field: OwnField;
  /** The project of the item: its short name. */
  readonly project: string | undefined;
  /** The issue, when the item is one: its ID. */
  readonly issue?: string | undefined;
  readonly value: unknown;
  readonly editable: boolean;
  readonly onCommit: (field: OwnField, value: Sent) => void;
}

/** One field: its name and value, or its editor where the person may change it. */
function FieldLine({ field, project, issue, value, editable, onCommit }: FieldLineProps) {
  if (!editable) {
    return (
      <div {...partOf(part.stack)}>
        <p {...partOf(part.fieldLabel)}>{field.name}</p>
        <ValueView field={field} value={value} />
      </div>
    );
  }
  return (
    <ValueEditor
      field={field}
      project={project}
      issue={issue}
      value={value}
      onCommit={(sent) => {
        onCommit(field, sent);
      }}
    />
  );
}

/**
 * The own fields in the fields panel of the issue page (CF-06, UX §4.4): each field the person sees,
 * in its look, and an editor where the field is theirs to change. A field the person may not see is
 * not in the issue and so not here. The page saves a change and shows what the server refuses.
 * Long texts have their own section under the description.
 */
export function OwnFieldsPanel({ issue, mask, onChange }: IssueBlockProps) {
  const project = issue.project?.short_name;
  const fields = useFields({ project });
  const values: Values = issue.own_fields ?? {};
  return (
    <div {...partOf(part.stack)}>
      {fields.isError ? <Alert tone={alertTone.danger}>{messageOf(fields.error)}</Alert> : null}
      {issueFieldsOf(fields.data?.fields ?? [], issue)
        .filter(
          (field) => field.kind !== "long-text" && (mask.sees(field.id) || mask.changes(field.id)),
        )
        .map((field) => (
          <FieldLine
            key={lineKey(field, values[field.id])}
            field={field}
            project={project}
            issue={issue.id ?? undefined}
            value={values[field.id]}
            editable={mask.changes(field.id)}
            onCommit={(each, value) => {
              void onChange({ fields: { [each.id]: value } });
            }}
          />
        ))}
    </div>
  );
}

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
  const values: Values = project.own_fields;
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
