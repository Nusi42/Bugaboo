import { t } from "@bugaboo/messages";
import { SelectField, TextField } from "@bugaboo/ui";

import { FormNotice, FormSections, type FormView } from "../layouts/index.ts";
import type { NewIssueValues } from "./new-issue-form.ts";
import { NewIssueField } from "./NewIssueField.tsx";

export interface NewIssueFieldsProps {
  readonly values: NewIssueValues;
  /** The issue types that can be offered: the ones that are not retired. */
  readonly types: readonly { readonly id: string; readonly name: string }[];
  /** The project it is made in: its short name. */
  readonly project: string;
  /** Which fields the layout of the form asks this person for. */
  readonly layout: FormView;
  /** The refusal of the server about one field, shown at that field. */
  readonly refusal?: { readonly field: string; readonly message: string } | undefined;
  readonly onChange: (values: NewIssueValues) => void;
}

/**
 * The fields of the form of a new issue (UX §4.5): the type and the title, which every form has,
 * and then the sections of the layout of the project and the type (CFG-01). A field is on the form
 * only when the layout lists it for one of the person's roles and its level lets them fill it in
 * (the server decides, so the form never asks for what would be refused).
 */
export function NewIssueFields({
  values,
  types,
  project,
  layout,
  refusal,
  onChange,
}: NewIssueFieldsProps) {
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
        label={t("fields.label-required", { field: t("issue-field.title") })}
        requiredMessage={t("error.field.required")}
        value={values.title}
        onChange={(title) => {
          onChange({ ...values, title });
        }}
      />
      <FormNotice form={layout} />
      <FormSections
        sections={layout.sections}
        titles
        render={(field) => (
          <NewIssueField
            field={field}
            values={values}
            project={project}
            problem={refusal?.field === field ? refusal.message : undefined}
            onChange={onChange}
          />
        )}
      />
    </>
  );
}
