import { t } from "@bugaboo/messages";
import { fieldKind, Switch, TextField, partOf, part } from "@bugaboo/ui";

import { OwnFieldInput } from "../fields/index.ts";
import type { EntryValues } from "./entry-values.ts";
import { useEntryFields } from "./use-entry-fields.ts";
import type { EntryDefaults } from "./use-entry-defaults.ts";

export interface EntryFormProps {
  /** The issue the time is on: its project's short name and the ID of its type decide the fields. */
  readonly issue: { readonly project: string; readonly issueType: string | undefined };
  readonly defaults: EntryDefaults;
  readonly values: EntryValues;
  readonly onChange: (values: EntryValues) => void;
  /** What the server said was wrong, by the ID of the field, or `note`: shown at that box. */
  readonly problems?: Readonly<Record<string, string>>;
}

/**
 * The boxes of a time entry that are the same wherever time is booked (TIME-01, TIME-02, TIME-13):
 * *Activity*, *Cost center* and the other fields of time entries that must be filled in, started
 * with the defaults of the issue; the note; and *Billable* while the organization offers it. The
 * window that stops the timer, the window of *Add time* and, in the timesheet, the details of a row
 * all show this form and add their own date and length above it.
 */
/** No problem at any box. */
const noProblems: Readonly<Record<string, string>> = {};

export function EntryForm({
  issue,
  defaults,
  values,
  onChange,
  problems = noProblems,
}: EntryFormProps) {
  const fields = useEntryFields(issue.project, issue.issueType);
  return (
    <div {...partOf(part.stack)}>
      {fields.length === 0 ? null : <p {...partOf(part.muted)}>{t("time.entry.from-issue")}</p>}
      {fields.map((field) => (
        <OwnFieldInput
          key={field.id}
          field={field.id}
          project={issue.project}
          issueType={issue.issueType}
          value={values.fields[field.id]}
          problem={problems[field.id]}
          onChange={(value) => {
            onChange({ ...values, fields: { ...values.fields, [field.id]: value } });
          }}
        />
      ))}
      <TextField
        label={t("time.entry.note")}
        kind={fieldKind.multiline}
        value={values.note}
        errorMessage={problems["note"]}
        onChange={(note) => {
          onChange({ ...values, note });
        }}
      />
      {defaults.billable_offered ? (
        <Switch
          label={t("time.entry.billable")}
          isSelected={values.billable}
          onChange={(billable) => {
            onChange({ ...values, billable });
          }}
        />
      ) : null}
    </div>
  );
}
