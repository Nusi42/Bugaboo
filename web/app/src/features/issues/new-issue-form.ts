import type { Schema } from "@bugaboo/api-client";
import { priorityWords } from "@bugaboo/api-client";

import type { NamedPerson } from "../../shared/person.ts";

/** What a person has filled in on the form of a new issue. */
export interface NewIssueValues {
  /** The ID of the issue type; empty until one is picked. */
  readonly issueType: string;
  readonly title: string;
  readonly description: string;
  readonly priority: Schema<"Priority">;
  /** The people it is assigned to. */
  readonly assignedTo: readonly NamedPerson[];
  /** The values of own fields as the API takes them, by the key of the field. */
  readonly fields: Readonly<Record<string, unknown>>;
}

/** The form before anything is filled in. */
export const emptyForm: NewIssueValues = {
  issueType: "",
  title: "",
  description: "",
  priority: "normal",
  assignedTo: [],
  fields: {},
};

/** Whether anything was written: only then is a draft worth keeping. */
export function hasWriting(values: NewIssueValues): boolean {
  return (
    values.title.trim() !== "" ||
    values.description.trim() !== "" ||
    values.assignedTo.length > 0 ||
    Object.keys(values.fields).length > 0
  );
}

/** The values as the text of a draft. */
export function encodeDraft(values: NewIssueValues): string {
  return JSON.stringify(values);
}

/** The people of a draft: a list of IDs with names, and nothing for what is not one. */
function peopleOf(found: unknown): readonly NamedPerson[] {
  if (!Array.isArray(found)) {
    return [];
  }
  return found.flatMap((each: unknown): NamedPerson[] => {
    if (typeof each !== "object" || each === null) {
      return [];
    }
    const id: unknown = Reflect.get(each, "id");
    const name: unknown = Reflect.get(each, "name");
    return typeof id === "string" && typeof name === "string" ? [{ id, name }] : [];
  });
}

/** The values of own fields of a draft: an object, or nothing. */
function fieldsOf(found: unknown): Readonly<Record<string, unknown>> {
  return typeof found === "object" && found !== null && !Array.isArray(found)
    ? Object.fromEntries(Object.entries(found))
    : {};
}

/** The values from the text of a draft; nothing for a text that is no draft of this form. */
export function decodeDraft(text: string): NewIssueValues | undefined {
  try {
    const found: unknown = JSON.parse(text);
    if (typeof found !== "object" || found === null) {
      return undefined;
    }
    const read = (name: string): unknown => Reflect.get(found, name);
    const [issueType, title, description] = [read("issueType"), read("title"), read("description")];
    const priority = priorityWords.find((word) => word === read("priority"));
    // A draft kept before the form had these two has none: they start empty.
    return typeof issueType === "string" &&
      typeof title === "string" &&
      typeof description === "string" &&
      priority !== undefined
      ? {
          issueType,
          title,
          description,
          priority,
          assignedTo: peopleOf(read("assignedTo")),
          fields: fieldsOf(read("fields")),
        }
      : undefined;
  } catch (error) {
    if (error instanceof SyntaxError) {
      return undefined;
    }
    throw error;
  }
}

/** Whether the form can be sent: it has a title and a type. */
export function canCreate(values: NewIssueValues): boolean {
  return values.title.trim() !== "" && values.issueType !== "";
}

/** The values of own fields for which `keep` says yes. */
function fieldsWhere(
  fields: Readonly<Record<string, unknown>>,
  keep: (key: string, value: unknown) => boolean,
): Readonly<Record<string, unknown>> {
  const kept: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(fields)) {
    if (keep(key, value)) {
      kept[key] = value;
    }
  }
  return kept;
}

/** The values with what the form does not show to this person put back to nothing. */
export function forForm(values: NewIssueValues, shows: (field: string) => boolean): NewIssueValues {
  return {
    issueType: values.issueType,
    title: values.title,
    description: shows("description") ? values.description : emptyForm.description,
    priority: shows("priority") ? values.priority : emptyForm.priority,
    assignedTo: shows("assigned-to") ? values.assignedTo : emptyForm.assignedTo,
    fields: fieldsWhere(values.fields, (key) => shows(key)),
  };
}

/** The body that makes the issue: a priority only when the person moved it from the usual one. */
export function bodyOf(values: NewIssueValues) {
  const fields = fieldsWhere(values.fields, (_, value) => value !== null);
  return {
    issue_type: values.issueType,
    title: values.title.trim(),
    ...(values.description.trim() === "" ? {} : { description: values.description }),
    ...(values.priority === emptyForm.priority ? {} : { priority: values.priority }),
    ...(values.assignedTo.length === 0
      ? {}
      : { assigned_to: values.assignedTo.map((person) => person.id) }),
    ...(Object.keys(fields).length === 0 ? {} : { fields }),
  };
}
