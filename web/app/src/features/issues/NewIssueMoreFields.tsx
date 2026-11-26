import { t } from "@bugaboo/messages";
import { MarkdownEditor, SelectField } from "@bugaboo/ui";

import { useImageHosts } from "../../shared/image-hosts.ts";
import { markdownLabels } from "../../shared/markdown-labels.ts";
import { LengthBox } from "../fields/index.ts";
import { priorityChoices } from "./issue-choices.ts";
import { withMistake } from "./new-issue-form.ts";
import type { NewIssueFieldProps } from "./NewIssueField.tsx";

/** *Estimated time* on the form: a length the person types; a text that is none holds the form back. */
export function EstimateField({
  values,
  problem,
  onChange,
}: Omit<NewIssueFieldProps, "field" | "project">) {
  return (
    <LengthBox
      name={t("issue-field.estimated-time")}
      minutes={values.estimatedTime ?? undefined}
      inRow={false}
      problem={problem}
      onCommit={(estimatedTime) => {
        onChange({ ...withMistake(values, "estimated-time", false), estimatedTime });
      }}
      onMistake={() => {
        onChange(withMistake(values, "estimated-time", true));
      }}
    />
  );
}

/** *Description* on the form: the editor of text, with the hosts its images may come from. */
export function DescriptionField({
  values,
  onChange,
}: Pick<NewIssueFieldProps, "values" | "onChange">) {
  const hosts = useImageHosts();
  return (
    <MarkdownEditor
      label={t("issue-field.description")}
      labels={markdownLabels()}
      value={values.description}
      onChange={(description) => {
        onChange({ ...values, description });
      }}
      imageHosts={hosts}
    />
  );
}

/** *Priority* on the form. */
export function PriorityField({
  values,
  onChange,
}: Pick<NewIssueFieldProps, "values" | "onChange">) {
  return (
    <SelectField
      label={t("issue-field.priority")}
      choices={priorityChoices}
      value={values.priority}
      onChange={(priority) => {
        onChange({ ...values, priority });
      }}
    />
  );
}
