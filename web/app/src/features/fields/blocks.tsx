import { Alert, alertTone, NameValueList, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import type { IssueBlockProps } from "../issues/index.ts";
import { FormSections, isPlain, onlyFields } from "../layouts/index.ts";
import { issueFieldsOf, useFields, type OwnField, type FieldValues } from "./fields.ts";
import { FieldLine } from "./FieldLine.tsx";

/**
 * The own fields the person sees on the issue page, a row each (CF-06, UX §4.4): a field the person
 * may not see is not in the issue and so not here, and a field the layout does not show is left out.
 * The page saves a change and shows what the server refuses. Long texts have their own section
 * under the description.
 */
function useOwnFieldLines({ issue, mask, form, rowStatus, onChange }: IssueBlockProps) {
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
  return {
    error: fields.isError ? fields.error : null,
    shown,
    line,
    sections,
    ready: form.ready,
  };
}

/**
 * The own fields as rows of the one list of the Details, when the layout puts them under one run:
 * no titled sections and no tabs (the other case is `OwnFieldGroups`).
 */
export function OwnFieldRows(props: IssueBlockProps) {
  const { shown, line, sections } = useOwnFieldLines(props);
  if (!isPlain(sections)) {
    return null;
  }
  const order = sections.flatMap((section) => section.fields);
  return order.map((key) => {
    const field = shown.find((each) => each.id === key);
    return field === undefined ? null : line(field);
  });
}

/**
 * The own fields of a layout with titled sections or tabs, after the list of the Details, each
 * section with a list of its own; and what went wrong reading the fields. Nothing for a layout that
 * `OwnFieldRows` holds.
 */
export function OwnFieldGroups(props: IssueBlockProps) {
  const { error, shown, line, sections, ready } = useOwnFieldLines(props);
  const grouped = ready && !isPlain(sections);
  if (error === null && !grouped) {
    return null;
  }
  return (
    <div {...partOf(part.stack)}>
      {error === null ? null : <Alert tone={alertTone.danger}>{messageOf(error)}</Alert>}
      {grouped ? (
        <FormSections
          sections={sections}
          titles={sections.length > 1}
          render={(key) => {
            const field = shown.find((each) => each.id === key);
            return field === undefined ? null : <NameValueList>{line(field)}</NameValueList>;
          }}
        />
      ) : null}
    </div>
  );
}
