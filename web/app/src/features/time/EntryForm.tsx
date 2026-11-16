import { t } from "@bugaboo/messages";
import { fieldKind, Switch, TextField, partOf, part } from "@bugaboo/ui";

import { MistakeNote } from "../../shared/MistakeNote.tsx";
import type { FormMistakes } from "../../shared/mistakes.ts";
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
  /**
   * The boxes of the whole form that hold no value yet (`useMistakes`): a box of a field tells it
   * here, and the form says why it is held at its end.
   */
  readonly mistakes: FormMistakes;
}

/** No problem at any box. */
const noProblems: Readonly<Record<string, string>> = {};

/** One own field of time entries in the form, which tells the form when its box holds no value yet. */
function EntryField({
  field,
  form,
}: {
  readonly field: string;
  readonly form: Pick<EntryFormProps, "issue" | "values" | "onChange"> & {
    readonly problems: Readonly<Record<string, string>>;
    readonly mark: FormMistakes["mark"];
  };
}) {
  const { issue, values, problems, onChange, mark } = form;
  return (
    <OwnFieldInput
      field={field}
      project={issue.project}
      issueType={issue.issueType}
      value={values.fields[field]}
      problem={problems[field]}
      onChange={(value) => {
        mark(field)(false);
        onChange({ ...values, fields: { ...values.fields, [field]: value } });
      }}
      onMistake={() => {
        mark(field)(true);
      }}
    />
  );
}

/**
 * The boxes of a time entry that are the same wherever time is booked (TIME-01, TIME-02, TIME-13):
 * *Activity*, *Cost center* and the other fields of time entries that must be filled in, started
 * with the defaults of the issue; the note; and *Billable* while the organization offers it. The
 * window that stops the timer, the window of *Add time* and, in the timesheet, the details of a row
 * all show this form and add their own date and length above it.
 */
export function EntryForm({
  issue,
  defaults,
  values,
  onChange,
  problems = noProblems,
  mistakes,
}: EntryFormProps) {
  const { mark } = mistakes;
  const fields = useEntryFields(issue.project, issue.issueType);
  return (
    <div {...partOf(part.stack)}>
      {fields.length === 0 ? null : <p {...partOf(part.muted)}>{t("time.entry.from-issue")}</p>}
      {fields.map((field) => (
        <EntryField
          key={field.id}
          field={field.id}
          form={{ issue, values, problems, onChange, mark }}
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
      <MistakeNote mistakes={mistakes.list} />
    </div>
  );
}
