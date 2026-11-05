import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, TextField } from "@bugaboo/ui";

import { LengthBox } from "../fields/index.ts";
import type { EntryIssue } from "./entry-issue.ts";
import { EntryForm } from "./EntryForm.tsx";
import type { AddTimeState } from "./use-add-time-state.ts";
import type { EntryProblem } from "./use-add-time.ts";
import type { EntryDefaults } from "./use-entry-defaults.ts";

/** The widest span of days a day box takes: the whole calendar. */
const CALENDAR = { min: "0001-01-01", max: "9999-12-31" };

export interface AddTimeFormProps {
  /** The ID of the form, for the button of the footer that sends it. */
  readonly id: string;
  readonly issue: EntryIssue;
  readonly defaults: EntryDefaults;
  readonly state: AddTimeState;
  readonly problem: EntryProblem | undefined;
  readonly onSubmit: () => void;
}

/** The boxes of *Add time*: the date, the length, and the form every time entry has. */
export function AddTimeForm({ id, issue, defaults, state, problem, onSubmit }: AddTimeFormProps) {
  return (
    <Form id={id} onSubmit={onSubmit}>
      {problem !== undefined && problem.field === undefined ? (
        <Alert tone={alertTone.danger}>{problem.text}</Alert>
      ) : null}
      <TextField
        label={t("time.entry.date")}
        date={CALENDAR}
        value={state.date}
        onChange={state.setDate}
      />
      <LengthBox
        name={t("time.entry.length")}
        minutes={state.minutes ?? undefined}
        inRow={false}
        onCommit={state.setMinutes}
      />
      <EntryForm
        issue={issue}
        defaults={defaults}
        values={state.values}
        onChange={state.setValues}
        problems={problem?.field === undefined ? {} : { [problem.field]: problem.text }}
      />
    </Form>
  );
}
