import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

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
  return (
    <>
      <p {...partOf(part.text)}>
        {single
          ? `${words.join(". ")}.`
          : t("agent-work.missing-many", { issue, parts: words.join(", ") })}
      </p>
      {single ? <p {...partOf(part.muted)}>{t("agent-work.missing-ends")}</p> : null}
    </>
  );
}

/**
 * What the brief lacks, named and never in the way (UX §4.17): *No steps to reproduce*, *No Done
 * when*. The agent gets what is there and the issue can be handed over anyway. For ticked issues
 * each issue that lacks something is named, as far as the window reads them.
 */
export function BriefMissing({ issues }: { readonly issues: readonly string[] }) {
  const single = issues.length === 1;
  return (
    <div {...partOf(part.agentWorkMissing)}>
      {issues.slice(0, MOST_READ).map((issue) => (
        <MissingOf key={issue} issue={issue} single={single} />
      ))}
      {single ? null : <p {...partOf(part.muted)}>{t("agent-work.missing-ends-many")}</p>}
    </div>
  );
}
