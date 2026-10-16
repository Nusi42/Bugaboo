import { Alert, alertTone, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import type { IssueBlockProps } from "../issues/index.ts";
import { FormSections, onlyFields } from "../layouts/index.ts";
import { issueFieldsOf, useFields, type OwnField, type FieldValues } from "./fields.ts";
import { FieldLine } from "./FieldLine.tsx";

/**
 * The own fields in the Details of the issue page (CF-06, UX §4.4): each field the person sees,
 * in its look, and an editor where the field is theirs to change. A field the person may not see is
 * not in the issue and so not here. The page saves a change and shows what the server refuses.
 * Long texts have their own section under the description.
 */
export function OwnFieldRows({ issue, mask, form, rowStatus, onChange }: IssueBlockProps) {
  const project = issue.project?.short_name;
  const fields = useFields({ project });
  const values: FieldValues = issue.own_fields ?? {};
  const shown = issueFieldsOf(fields.data?.fields ?? [], issue).filter(
    (field) =>
      field.kind !== "long-text" &&
      (mask.sees(field.id) || mask.changes(field.id)) &&
      form.shows(field.id),
  );
  const line = (field: OwnField) => (
    <FieldLine
      key={field.id}
      field={field}
      project={project}
      issue={issue.id ?? undefined}
      value={values[field.id]}
      editable={mask.changes(field.id)}
      status={rowStatus(field.id)}
      onCommit={(each, value) => {
        void onChange({ fields: { [each.id]: value } });
      }}
    />
  );
  const sections = onlyFields(form.sections, (key) => shown.some((field) => field.id === key));
  return (
    <div {...partOf(part.stack)}>
      {fields.isError ? <Alert tone={alertTone.danger}>{messageOf(fields.error)}</Alert> : null}
      {form.ready ? (
        <FormSections
          sections={sections}
          titles={sections.length > 1}
          render={(key) => {
            const field = shown.find((each) => each.id === key);
            return field === undefined ? null : line(field);
          }}
        />
      ) : (
        shown.map((field) => line(field))
      )}
    </div>
  );
}
