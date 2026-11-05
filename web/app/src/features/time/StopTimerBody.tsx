import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Length,
  SentenceWithValue,
  partOf,
  part,
} from "@bugaboo/ui";

import { minutesText } from "../../shared/own-value-text.ts";
import type { EntryIssue } from "./entry-issue.ts";
import type { EntryValues } from "./entry-values.ts";
import { EntryForm } from "./EntryForm.tsx";
import { askTime } from "./time-asks.ts";
import type { EntryProblem } from "./use-add-time.ts";
import type { EntryDefaults } from "./use-entry-defaults.ts";

export interface StopTimerBodyProps {
  readonly issue: EntryIssue;
  readonly defaults: EntryDefaults;
  /** Whole minutes the timer has run; under one the window says nothing will be saved. */
  readonly minutes: number;
  readonly values: EntryValues;
  readonly onChange: (values: EntryValues) => void;
  readonly problem: EntryProblem | undefined;
}

/** What the window says about the time: *You worked 42m on …*, or that nothing will be saved. */
function Worked({ issue, minutes }: Pick<StopTimerBodyProps, "issue" | "minutes">) {
  if (minutes === 0) {
    return <p {...partOf(part.text)}>{t("time.stop.nothing", { issue: issue.key })}</p>;
  }
  const length = minutesText(minutes);
  return (
    <p {...partOf(part.text)}>
      <SentenceWithValue
        sentence={t("time.stop.worked", { length, title: issue.title })}
        value={length}
      >
        {(text) => <Length text={text} />}
      </SentenceWithValue>
    </p>
  );
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
      <Worked issue={issue} minutes={minutes} />
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
