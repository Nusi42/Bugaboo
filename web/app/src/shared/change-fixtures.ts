import type { Schema } from "@bugaboo/api-client";

type Value = Schema<"ChangeValueBody">;

/**
 * What the tests of the history and of the notifications write for a change of one field: the field
 * with the two sides flat (`before_text`, `after_names`), which is shorter to read in a test than the
 * nested `change` an item carries. `withChange` builds the one shape of the API from it.
 */
export interface FlatChange {
  readonly field?: string;
  readonly field_name?: string;
  readonly field_kind?: Schema<"FieldType">;
  readonly before_text?: string;
  readonly after_text?: string;
  readonly before_value?: unknown;
  readonly after_value?: unknown;
  readonly before_names?: readonly string[];
  readonly after_names?: readonly string[];
  readonly before_people?: Extract<Value, { shown: "people" }>["people"];
  readonly after_people?: Extract<Value, { shown: "people" }>["people"];
  readonly before_issue?: Schema<"IssueRef">;
  readonly after_issue?: Schema<"IssueRef">;
  readonly before_project?: Schema<"ProjectRef">;
  readonly after_project?: Schema<"ProjectRef">;
}

/** One side of a flat change, as the value of the API. */
interface Side {
  readonly text?: string | undefined;
  readonly value?: unknown;
  readonly kind?: Schema<"FieldType"> | undefined;
  readonly names?: readonly string[] | undefined;
  readonly people?: Extract<Value, { shown: "people" }>["people"] | undefined;
  readonly issue?: Schema<"IssueRef"> | undefined;
  readonly project?: Schema<"ProjectRef"> | undefined;
}

function valueOf(side: Side): Value | undefined {
  if (side.text !== undefined) {
    return { shown: "text", text: side.text };
  }
  if (side.value !== undefined && side.kind !== undefined) {
    return { shown: "typed", kind: side.kind, value: side.value };
  }
  if (side.names !== undefined) {
    return { shown: "names", names: [...side.names] };
  }
  if (side.people !== undefined) {
    return { shown: "people", people: side.people };
  }
  if (side.project !== undefined) {
    return { shown: "project", project: side.project };
  }
  return side.issue === undefined ? undefined : { shown: "issue", issue: side.issue };
}

/**
 * The members of an item without the flat change, and the `change` made from it when the test named
 * a field.
 */
export function withChange<T extends FlatChange>(overrides: T) {
  const {
    field,
    field_name,
    field_kind,
    before_text,
    after_text,
    before_value,
    after_value,
    before_names,
    after_names,
    before_people,
    after_people,
    before_issue,
    after_issue,
    before_project,
    after_project,
    ...rest
  } = overrides;
  if (field === undefined) {
    return rest;
  }
  const before = valueOf({
    text: before_text,
    value: before_value,
    kind: field_kind,
    names: before_names,
    people: before_people,
    issue: before_issue,
    project: before_project,
  });
  const after = valueOf({
    text: after_text,
    value: after_value,
    kind: field_kind,
    names: after_names,
    people: after_people,
    issue: after_issue,
    project: after_project,
  });
  const change: Schema<"FieldChangeBody"> = {
    field,
    ...(field_name === undefined ? {} : { name: field_name }),
    ...(before === undefined ? {} : { before }),
    ...(after === undefined ? {} : { after }),
  };
  return { ...rest, change };
}
