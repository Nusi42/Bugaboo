import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part } from "@bugaboo/ui";

import type { EntryIssue } from "./entry-issue.ts";
import type { EntryValues } from "./entry-values.ts";
import { EntryForm } from "./EntryForm.tsx";
import { StopTimerSaid } from "./StopTimerSaid.tsx";
import { askTime } from "./time-asks.ts";
import type { EntryProblem } from "./use-add-time.ts";
import type { EntryDefaults } from "./use-entry-defaults.ts";

export interface StopTimerBodyProps {
  readonly issue: EntryIssue;
  readonly defaults: EntryDefaults;
  /** Whole minutes the timer has run: at least one, or the window is not shown. */
  readonly minutes: number;
  readonly values: EntryValues;
  readonly onChange: (values: EntryValues) => void;
  readonly problem: EntryProblem | undefined;
}

/** A timer that ran longer than one entry can be: the server's sentence, and the way to change the start. */
function TooLong({ problem }: { readonly problem: EntryProblem }) {
  return (
    <Alert tone={alertTone.danger}>
      <span>{problem.text}</span>
      {problem.code === "timer.too-long" ? (
        <Button
          look={buttonLook.quiet}
          onPress={() => {
            askTime({ kind: "start-time" });
          }}
        >
          {t("time.timer.change-start")}
        </Button>
      ) : null}
    </Alert>
  );
}

/** The inside of the window that stops the timer. */
export function StopTimerBody({
  issue,
  defaults,
  minutes,
  values,
  onChange,
  problem,
}: StopTimerBodyProps) {
  return (
    <div {...partOf(part.stack)}>
      <StopTimerSaid issue={issue} minutes={minutes} />
      {problem === undefined || problem.field !== undefined ? null : <TooLong problem={problem} />}
      <EntryForm
        issue={issue}
        defaults={defaults}
        values={values}
        onChange={onChange}
        problems={problem?.field === undefined ? {} : { [problem.field]: problem.text }}
      />
    </div>
  );
}
