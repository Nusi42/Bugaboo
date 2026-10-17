import { useState } from "react";

import { fieldOf } from "../../shared/errors.ts";
import { askedFieldsOf, askKeyOf, askValuesOf, type OfferedChange } from "./ask-for.ts";
import { emptyForm, type NewIssueValues } from "./new-issue-form.ts";
import type { IssueView } from "./slots.ts";
import type { IssueChanges } from "./use-issue-changes.ts";

/** What the window of *Ask for* keeps: the values, whether the save is on its way, and why it was refused. */
export function useAskFor(
  issue: IssueView,
  change: OfferedChange,
  changes: IssueChanges,
  onDone: () => void,
) {
  const [values, setValues] = useState<NewIssueValues>({
    ...emptyForm,
    issueType: issue.issue_type?.id ?? "",
  });
  const [pending, setPending] = useState(false);
  const [refused, setRefused] = useState<unknown>();
  const asked = askedFieldsOf(change.needs);
  const field = fieldOf(refused);
  return {
    values,
    /** A new value takes back the refusal that was about the old one. */
    fill: (next: NewIssueValues): void => {
      setValues(next);
      setRefused(undefined);
    },
    pending,
    asked,
    refused,
    /** The field the refusal is about, when the window shows that field. */
    refusedField: asked.some((each) => askKeyOf(each) === field) ? field : undefined,
    send: (): void => {
      setPending(true);
      setRefused(undefined);
      changes
        .ask(askValuesOf(change.to, change.needs, values))
        .then(onDone)
        .catch((error: unknown) => {
          setRefused(error);
          setPending(false);
        });
    },
  };
}
