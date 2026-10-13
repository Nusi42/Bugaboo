import { priorityWords, statusWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { known } from "../../shared/present.ts";
import { AssignedFact } from "./AssignedFact.tsx";
import { ChoiceFact } from "./Fact.tsx";
import type { IssueValues } from "./issue-values.ts";
import type { IssueBlockProps, IssueFieldMask, IssueView } from "./slots.ts";

export interface FactsBarProps {
  readonly issue: IssueView;
  readonly mask: IssueFieldMask;
  /** Which of the fields the layout of the issue page shows to this person. */
  readonly form: IssueBlockProps["form"];
  readonly onChange: (values: IssueValues) => void;
}

/** The key that moves the focus to the status. */
const statusShortcut = "change-status" as const;

const statusChoices = statusWords.map((word) => ({ value: word, label: t(`status.${word}`) }));
const priorityChoices = priorityWords.map((word) => ({
  value: word,
  label: t(`priority.${word}`),
}));

/**
 * The most used fields under the title (UX §3.4): status, assigned to and priority. A field the
 * person may not see, or that the layout does not show them, is left out; one they may see and
 * not change says why.
 */
export function FactsBar(props: FactsBarProps) {
  const { issue, mask, form, onChange } = props;
  const [status, priority] = [known(issue.status), known(issue.priority)];
  return (
    <div {...partOf(part.issueFacts)}>
      {status === undefined || !form.shows("status") ? null : (
        <ChoiceFact
          shortcut={statusShortcut}
          name={t("issue-field.status")}
          choices={statusChoices}
          value={status}
          canChange={mask.changes("status")}
          onPick={(picked) => {
            onChange({ status: picked });
          }}
        />
      )}
      {known(issue.assigned_to) === undefined || !form.shows("assigned-to") ? null : (
        <AssignedFact {...props} />
      )}
      {priority === undefined || !form.shows("priority") ? null : (
        <ChoiceFact
          name={t("issue-field.priority")}
          choices={priorityChoices}
          value={priority}
          canChange={mask.changes("priority")}
          onPick={(picked) => {
            onChange({ priority: picked });
          }}
        />
      )}
    </div>
  );
}
