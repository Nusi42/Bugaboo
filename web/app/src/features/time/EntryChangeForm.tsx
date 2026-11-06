import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form } from "@bugaboo/ui";
import { useState } from "react";

import { LengthBox } from "../fields/index.ts";
import type { TimeEntry } from "./edits.ts";
import { EntryForm, type EntryFormProps } from "./EntryForm.tsx";
import type { EntryProblem } from "./use-add-time.ts";
import { valuesOfEntry } from "./use-change-entry.ts";

export interface EntryChangeFormProps {
  /** The ID of the form, for the button of the footer that sends it. */
  readonly id: string;
  readonly entry: TimeEntry;
  readonly issue: EntryFormProps["issue"];
  readonly defaults: EntryFormProps["defaults"];
  readonly problem: EntryProblem | undefined;
  /** Asked to save: the length and the values the boxes hold. */
  readonly onSubmit: (minutes: number, values: EntryFormProps["values"]) => void;
}

/**
 * The boxes of the details of one entry: its length (the box of *Add time*), and the form every
 * time entry has (the same `EntryForm` as *Add time* and *Stop timer*), started with what the entry
 * holds.
 */
export function EntryChangeForm({
  id,
  entry,
  issue,
  defaults,
  problem,
  onSubmit,
}: EntryChangeFormProps) {
  const [minutes, setMinutes] = useState<number>(entry.minutes);
  const [values, setValues] = useState<EntryFormProps["values"]>(() => valuesOfEntry(entry));
  return (
    <Form
      id={id}
      onSubmit={() => {
        onSubmit(minutes, values);
      }}
    >
      {problem !== undefined && problem.field === undefined ? (
        <Alert tone={alertTone.danger}>{problem.text}</Alert>
      ) : null}
      <LengthBox
        name={t("time.entry.length")}
        help={t("time.entry.length-hint")}
        minutes={minutes}
        inRow={false}
        onCommit={(committed) => {
          setMinutes(committed ?? entry.minutes);
        }}
      />
      <EntryForm
        issue={issue}
        defaults={defaults}
        values={values}
        onChange={setValues}
        problems={problem?.field === undefined ? {} : { [problem.field]: problem.text }}
      />
    </Form>
  );
}
