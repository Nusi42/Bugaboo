import type { Schema } from "@bugaboo/api-client";
import { Alert, alertTone, Card } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshFields } from "../../shared/query-keys.ts";
import { useFields, type OwnField } from "./fields.ts";
import type { Sent } from "./ValueEditor.tsx";

type Project = Schema<"ProjectDetailBody">;

/** What a card of fields of a project hands its lines: the changeable fields and the way to change one. */
export interface ProjectFieldsContext {
  /** Whether the person may change the field in this project. */
  readonly mayChange: (field: OwnField) => boolean;
  /** The value the project holds for the field; nothing when it holds none. */
  readonly valueOf: (field: OwnField) => unknown;
  /** Sends a new value of a field of the project, or takes it away with `null`. */
  readonly commit: (field: OwnField, value: Sent) => void;
}

export interface ProjectFieldsCardProps {
  readonly project: Project;
  readonly heading: string;
  readonly intro?: string | undefined;
  /** Which of the fields of the organization the card shows. */
  readonly wanted: (field: OwnField, context: ProjectFieldsContext) => boolean;
  /** The lines of the fields, in the container of the caller. */
  readonly children: (fields: readonly OwnField[], context: ProjectFieldsContext) => ReactNode;
}

/**
 * The card of the own fields of a project that a screen shows (CF-08, TIME-13): the fields it
 * wants, with one way to change the value of a field of the project and one place that tells what
 * went wrong. Not drawn when no field is wanted. The project's overview and the time defaults of its
 * settings are two cards of this one part.
 */
export function ProjectFieldsCard({
  project,
  heading,
  intro,
  wanted,
  children,
}: ProjectFieldsCardProps) {
  const fields = useFields({ project: project.project.short_name });
  const change = api.query.useMutation("patch", "/api/v1/projects/{project}", {
    onSuccess: refreshFields,
  });
  const context: ProjectFieldsContext = {
    mayChange: (field) => project.changeable_fields.some((key) => key === field.id),
    valueOf: (field) => project.own_fields[field.id],
    commit: (field, value) => {
      change.mutate({
        params: { path: { project: project.project.id } },
        body: { version: project.project.version, fields: { [field.id]: value } },
      });
    },
  };
  const shown = (fields.data?.fields ?? []).filter((field) => wanted(field, context));
  if (shown.length === 0) {
    return null;
  }
  return (
    <Card heading={heading} {...(intro === undefined ? {} : { intro })}>
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
      {children(shown, context)}
    </Card>
  );
}
