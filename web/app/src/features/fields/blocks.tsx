import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshFields } from "../../shared/query-keys.ts";
import { useFields, type Field } from "./fields.ts";
import { ValueEditor, type Sent } from "./ValueEditor.tsx";
import { ValueView } from "./ValueView.tsx";

/** What a block of own fields needs of the person's rights, as the issue page words them. */
export interface FieldMask {
  /** Whether the person sees the field at all. */
  readonly sees: (key: string) => boolean;
  /** Whether the person may change the field in this item. */
  readonly changes: (key: string) => boolean;
}

/** The values of the own fields of an item, by the key of the field, as the server sent them. */
type Values = Readonly<Record<string, unknown>>;

/** The change a block sends: the values of own fields to set, by key (`null` takes one away). */
export type OwnFieldsChange = Readonly<Record<string, unknown>>;

export interface OwnFieldsBlockProps {
  /** The issue as the person sees it. */
  readonly issue: Schema<"IssueItem">;
  /** What the person may change in this issue. */
  readonly mask: FieldMask;
  /** Sets values of the issue; rejects with the problem the server found. */
  readonly onChange: (change: OwnFieldsChange) => Promise<void>;
}

/** The own fields of an issue that the person sees: used in its project and its issue type. */
function fieldsOf(fields: readonly Field[], issue: Schema<"IssueItem">): readonly Field[] {
  const type = issue.issue_type?.id;
  return fields.filter(
    (field) =>
      field.applies_to === "issues" &&
      (field.scope.issue_types.length === 0 ||
        field.scope.issue_types.some((each) => each.id === type)),
  );
}

/** What makes a line start afresh: the field, the value it holds and the version of the item. */
function lineKey(field: Field, value: unknown, version = 0): string {
  return [field.id, JSON.stringify(value), version].join("|");
}

interface FieldLineProps {
  readonly field: Field;
  readonly value: unknown;
  readonly editable: boolean;
  readonly onCommit: (field: Field, value: Sent) => Promise<unknown>;
}

/** One field: its name and value, or its editor where the person may change it. */
function FieldLine({ field, value, editable, onCommit }: FieldLineProps) {
  const [problem, setProblem] = useState<string | undefined>();
  const [pending, setPending] = useState(false);
  if (!editable || field.kind === "long-text") {
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
      value={value}
      problem={problem}
      isPending={pending}
      onCommit={(sent) => {
        setPending(true);
        setProblem(undefined);
        void onCommit(field, sent)
          .catch((error: unknown) => {
            setProblem(messageOf(error));
          })
          .finally(() => {
            setPending(false);
          });
      }}
    />
  );
}

/**
 * The own fields in the fields panel of the issue page (CF-06, UX §4.4): each field the person sees,
 * in its look, and an editor where the field is theirs to change. A field the person may not see is
 * not in the issue and so not here. Long texts have their own section under the description.
 */
export function OwnFieldsPanel({ issue, mask, onChange }: OwnFieldsBlockProps) {
  const fields = useFields({ project: issue.project?.short_name });
  const shown = fieldsOf(fields.data?.fields ?? [], issue);
  const values: Values = issue.own_fields ?? {};
  return (
    <div {...partOf(part.stack)}>
      {fields.isError ? <Alert tone={alertTone.danger}>{messageOf(fields.error)}</Alert> : null}
      {shown
        .filter((field) => field.kind !== "long-text" && mask.sees(field.id))
        .map((field) => (
          <FieldLine
            key={lineKey(field, values[field.id])}
            field={field}
            value={values[field.id]}
            editable={mask.changes(field.id)}
            onCommit={(each, value) => onChange({ fields: { [each.id]: value } })}
          />
        ))}
    </div>
  );
}

/** The long texts among the own fields of an issue, as sections under the description. */
export function OwnFieldSections({ issue, mask }: Omit<OwnFieldsBlockProps, "onChange">) {
  const fields = useFields({ project: issue.project?.short_name });
  const values: Values = issue.own_fields ?? {};
  const sections = fieldsOf(fields.data?.fields ?? [], issue).filter(
    (field) => field.kind === "long-text" && mask.sees(field.id) && values[field.id] !== undefined,
  );
  return (
    <>
      {sections.map((field) => (
        <Card key={field.id} heading={field.name}>
          <ValueView field={field} value={values[field.id]} />
        </Card>
      ))}
    </>
  );
}

/**
 * The own fields of a project on its overview (CF-08): the same fields with the scope *Projects*,
 * in their look, and an editor for the person who may change the project.
 */
export function OwnFieldsOfProject({ project }: { readonly project: Schema<"ProjectDetailBody"> }) {
  const fields = useFields({ project: project.project.short_name });
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
          value={values[field.id]}
          editable={changeable.has(field.id)}
          onCommit={(each, value) =>
            change.mutateAsync({
              params: { path: { project: project.project.id } },
              body: { version: project.project.version, fields: { [each.id]: value } },
            })
          }
        />
      ))}
    </Card>
  );
}
