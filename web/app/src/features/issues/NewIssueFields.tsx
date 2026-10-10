import { priorityWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { MarkdownEditor, SelectField, TextField } from "@bugaboo/ui";

import { useImageHosts } from "../../shared/image-hosts.ts";
import { markdownLabels } from "../../shared/markdown-labels.ts";
import type { NewIssueValues } from "./new-issue-form.ts";

const priorityChoices = priorityWords.map((word) => ({
  value: word,
  label: t(`priority.${word}`),
}));

export interface NewIssueFieldsProps {
  readonly values: NewIssueValues;
  /** The issue types that can be offered: the ones that are not retired. */
  readonly types: readonly { readonly id: string; readonly name: string }[];
  readonly onChange: (values: NewIssueValues) => void;
}

/**
 * The fields of the form of a new issue (UX §4.5): the type, the title (required, and it says so
 * in words), the description and the priority. Which fields a role gets is decided in the layout
 * editor (WP-09); until then every person sees these.
 */
export function NewIssueFields({ values, types, onChange }: NewIssueFieldsProps) {
  const hosts = useImageHosts();
  return (
    <>
      <SelectField
        label={t("issues.type")}
        choices={types.map((type) => ({ value: type.id, label: type.name }))}
        value={values.issueType}
        onChange={(issueType) => {
          onChange({ ...values, issueType });
        }}
      />
      <TextField
        label={t("issues.title-required")}
        requiredMessage={t("validation.required")}
        value={values.title}
        onChange={(title) => {
          onChange({ ...values, title });
        }}
      />
      <MarkdownEditor
        label={t("issue-field.description")}
        labels={markdownLabels()}
        value={values.description}
        onChange={(description) => {
          onChange({ ...values, description });
        }}
        imageHosts={hosts}
      />
      <SelectField
        label={t("issue-field.priority")}
        choices={priorityChoices}
        value={values.priority}
        onChange={(priority) => {
          onChange({ ...values, priority });
        }}
      />
    </>
  );
}
