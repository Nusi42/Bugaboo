import { t } from "@bugaboo/messages";
import { NameValueList, partOf, part } from "@bugaboo/ui";

import type { IssueBlockProps } from "../issues/index.ts";
import { timeDefaultsOf, useFields, type FieldValues } from "./fields.ts";
import { FieldLine } from "./FieldLine.tsx";

/**
 * The defaults for time booked on the issue, after the list of the Details (ADR-0035, decision 1,
 * UX §4.4 *Accounting*): the fields of time entries, such as *Activity* and *Cost center*, whose
 * value on the issue is what every time entry booked on it starts with. They are no fields of the
 * issue, so no layout places them; they are shown and changed here with the levels of the fields.
 */
export function TimeDefaults({ issue, mask, rowStatus, onChange }: IssueBlockProps) {
  const project = issue.project?.short_name;
  const fields = useFields({ project });
  const values: FieldValues = issue.own_fields ?? {};
  const shown = timeDefaultsOf(fields.data?.fields ?? [], issue).filter(
    (field) => field.kind !== "long-text" && (mask.sees(field.id) || mask.changes(field.id)),
  );
  if (shown.length === 0) {
    return null;
  }
  const title = t("fields.time-defaults");
  return (
    <section {...partOf(part.formSection)} aria-label={title}>
      <h3 {...partOf(part.formSectionTitle)}>{title}</h3>
      <NameValueList>
        {shown.map((field) => (
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
        ))}
      </NameValueList>
    </section>
  );
}
