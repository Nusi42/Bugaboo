import { t } from "@bugaboo/messages";
import { Alert, alertTone, IssueId, partOf, part, SentenceWithValue } from "@bugaboo/ui";

import { listText } from "../../shared/list-text.ts";
import { missingText, useIssueAgentWork } from "./agent-work.ts";

/** How many issues of many ticked ones the window reads the brief of; the rest are handed over all the same. */
const MOST_READ = 20;

/** What one issue's brief lacks, from the agent work of the issue (the server works it out). */
function MissingOf({ issue, single }: { readonly issue: string; readonly single: boolean }) {
  const found = useIssueAgentWork(issue);
  const lacking = found.data?.brief_missing ?? [];
  if (lacking.length === 0) {
    return null;
  }
  const words = lacking.map((lack) => missingText(lack));
  const readable = found.data?.readable_id ?? "";
  return (
    <Alert tone={alertTone.warning}>
      {single ? (
        <>
          {words.map((word) => (
            <p key={word} {...partOf(part.text)}>
              {word}
            </p>
          ))}
          <p {...partOf(part.muted)}>{t("agent-work.missing-ends")}</p>
        </>
      ) : (
        <p {...partOf(part.text)}>
          <SentenceWithValue
            sentence={t("agent-work.missing-many", { issue: readable, parts: listText(words) })}
            value={readable}
          >
            {(id) => <IssueId id={id} />}
          </SentenceWithValue>
        </p>
      )}
    </Alert>
  );
}

/**
 * What the brief lacks, named and never in the way (UX §4.17): *No steps to reproduce*, *No Done
 * when*, in the warning part with its icon. The agent gets what is there and the issue can be
 * handed over anyway. For ticked issues each issue that lacks something is named by its readable
 * ID, as far as the window reads them.
 */
export function BriefMissing({ issues }: { readonly issues: readonly string[] }) {
  const single = issues.length === 1;
  return (
    <>
      {issues.slice(0, MOST_READ).map((issue) => (
        <MissingOf key={issue} issue={issue} single={single} />
      ))}
      {single ? null : <p {...partOf(part.muted)}>{t("agent-work.missing-ends-many")}</p>}
    </>
  );
}
