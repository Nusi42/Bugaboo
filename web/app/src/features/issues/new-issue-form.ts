import type { Schema } from "@bugaboo/api-client";
import { priorityWords } from "@bugaboo/api-client";

/** What a person has filled in on the form of a new issue. */
export interface NewIssueValues {
  /** The ID of the issue type; empty until one is picked. */
  readonly issueType: string;
  readonly title: string;
  readonly description: string;
  readonly priority: Schema<"Priority">;
}

/** The form before anything is filled in. */
export const emptyForm: NewIssueValues = {
  issueType: "",
  title: "",
  description: "",
  priority: "normal",
};

/** Whether anything was written: only then is a draft worth keeping. */
export function hasWriting(values: NewIssueValues): boolean {
  return values.title.trim() !== "" || values.description.trim() !== "";
}

/** The values as the text of a draft. */
export function encodeDraft(values: NewIssueValues): string {
  return JSON.stringify(values);
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
    return typeof issueType === "string" &&
      typeof title === "string" &&
      typeof description === "string" &&
      priority !== undefined
      ? { issueType, title, description, priority }
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

/** The body that makes the issue: a priority only when the person moved it from the usual one. */
export function bodyOf(values: NewIssueValues) {
  return {
    issue_type: values.issueType,
    title: values.title.trim(),
    ...(values.description.trim() === "" ? {} : { description: values.description }),
    ...(values.priority === emptyForm.priority ? {} : { priority: values.priority }),
  };
}
