import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Icon,
  iconName,
  iconSize,
  IssueId,
  partOf,
  part,
  SentenceWithValue,
} from "@bugaboo/ui";

import { missingSentence, useIssueAgentWork } from "./agent-work.ts";

/** How many issues of many ticked ones the window reads the brief of; the rest are handed over all the same. */
const MOST_READ = 20;

/** The warning of one issue: its icon, and the text beside it that starts under the icon's right edge. */
function Warning({ children }: { readonly children: React.ReactNode }) {
  return (
    <Alert tone={alertTone.warning}>
      <div {...partOf(part.agentWorkWarning)}>
        <Icon name={iconName.warning} size={iconSize.normal} />
        <p {...partOf(part.agentWorkWarningText)}>{children}</p>
      </div>
    </Alert>
  );
}

/** What one issue's brief lacks, from the agent work of the issue (the server works it out). */
function MissingOf({ issue, single }: { readonly issue: string; readonly single: boolean }) {
  const found = useIssueAgentWork(issue);
  const lacking = found.data?.brief_missing ?? [];
  if (lacking.length === 0) {
    return null;
  }
  const sentence = missingSentence(lacking);
  const readable = found.data?.readable_id ?? "";
  return (
    <Warning>
      {single ? (
        sentence
      ) : (
        <SentenceWithValue
          sentence={t("agent-work.missing-many", { issue: readable, parts: sentence })}
          value={readable}
        >
          {(id) => <IssueId id={id} />}
        </SentenceWithValue>
      )}
    </Warning>
  );
}

/**
 * What the brief lacks, named and never in the way (UX §4.17): one sentence (*No description, no
 * steps to reproduce and no Done when*) in the warning part with its icon, one layout for one
 * issue and for several. The agent gets what is there and the issue can be handed over anyway. For
 * ticked issues each issue that lacks something is named by its readable ID, as far as the window
 * reads them.
 */
export function BriefMissing({ issues }: { readonly issues: readonly string[] }) {
  const single = issues.length === 1;
  return (
    <>
      {issues.slice(0, MOST_READ).map((issue) => (
        <MissingOf key={issue} issue={issue} single={single} />
      ))}
      <p {...partOf(part.muted)}>
        {single ? t("agent-work.missing-ends") : t("agent-work.missing-ends-many")}
      </p>
    </>
  );
}
