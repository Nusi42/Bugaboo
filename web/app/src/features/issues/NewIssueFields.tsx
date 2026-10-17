import { t } from "@bugaboo/messages";
import { Alert, alertTone, SelectField, TextField } from "@bugaboo/ui";

import { FormNotice, FormSections, onlyFields, type FormView } from "../layouts/index.ts";
import type { NewIssueValues } from "./new-issue-form.ts";
import { NewIssueField } from "./NewIssueField.tsx";
import { ProjectChoice } from "./ProjectChoice.tsx";
import type { NewIssue } from "./use-new-issue.ts";

/** The field that stands with the title in the text column: the long one. */
const textField = "description";

export interface NewIssueTextProps {
  readonly values: NewIssueValues;
  /** The project it is made in: its short name. */
  readonly project: string;
  /** Which fields the layout of the form asks this person for. */
  readonly layout: FormView;
  readonly onChange: (values: NewIssueValues) => void;
}

/**
 * The text column of the form of a new issue (UX §4.5, §6.3): the title, which every form has, and
 * the description when the layout shows it. They stay at the left and, from 1200, in the middle.
 */
export function NewIssueText({ values, project, layout, onChange }: NewIssueTextProps) {
  return (
    <>
      <TextField
        label={t("fields.label-required", { field: t("issue-field.title") })}
        requiredMessage={t("error.field.required")}
        value={values.title}
        onChange={(title) => {
          onChange({ ...values, title });
        }}
      />
      {layout.shows(textField) ? (
        <NewIssueField field={textField} values={values} project={project} onChange={onChange} />
      ) : null}
    </>
  );
}

export interface NewIssueFieldsProps {
  readonly form: NewIssue;
  /** The refusal of the server about one field, shown at that field. */
  readonly refusal?: { readonly field: string; readonly message: string } | undefined;
}

/**
 * The fields of the form of a new issue (UX §4.5): the project and the type, which every form has,
 * and then the sections of the layout of the project and the type (CFG-01). A field is on the form
 * only when the layout lists it for one of the person's roles and its level lets them fill it in
 * (the server decides, so the form never asks for what would be refused).
 */
export function NewIssueFields({ form, refusal }: NewIssueFieldsProps) {
  const { project, values, layout } = form;
  return (
    <>
      <ProjectChoice name={project?.name} shortName={form.address} onPick={form.pick} />
      {project === undefined ? null : (
        <>
          {form.noType ? <Alert tone={alertTone.danger}>{t("issues.no-issue-type")}</Alert> : null}
          <SelectField
            label={t("issues.type")}
            choices={form.types.map((type) => ({ value: type.id, label: type.name }))}
            value={values.issueType}
            onChange={(issueType) => {
              form.change({ ...values, issueType });
            }}
          />
          <FormNotice form={layout} />
          <FormSections
            sections={onlyFields(layout.sections, (field) => field !== textField)}
            titles
            render={(field) => (
              <NewIssueField
                field={field}
                values={values}
                project={project.shortName}
                problem={refusal?.field === field ? refusal.message : undefined}
                onChange={form.change}
              />
            )}
          />
        </>
      )}
    </>
  );
}
