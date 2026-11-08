import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { useId } from "react";

import { IssueReadProblem } from "../../shared/IssueReadProblem.tsx";
import type { IssueBlockProps } from "../issues/index.ts";
import { useIssueAgentWork, type BriefLack, type Proposal, type Work } from "./agent-work.ts";
import { Proposals } from "./Proposals.tsx";
import { WorkPiece } from "./WorkPiece.tsx";

/**
 * The agent work of the issue (AI-33, UX §4.17), a block of its own under the description and apart
 * from the comments: a piece for each hand-over, newest first, and the proposals of standing rules
 * that wait for a person. An issue no agent works on has no block, and takes no room.
 */
export function AgentWorkBlock({
  readableId,
  issue,
}: Pick<IssueBlockProps, "readableId" | "issue">) {
  const heading = useId();
  const found = useIssueAgentWork(readableId);
  if (found.isError) {
    return <IssueReadProblem error={found.error} />;
  }
  const all = found.data;
  if (all === undefined || (all.work.length === 0 && all.proposals.length === 0)) {
    return null;
  }
  const works: readonly Work[] = all.work;
  const proposals: readonly Proposal[] = all.proposals;
  const lacking: readonly BriefLack[] = all.brief_missing;
  const names = Object.fromEntries(works.map((work) => [work.agent, work.agent_name]));
  return (
    <section {...partOf(part.agentWork)} aria-labelledby={heading}>
      <h2 id={heading} {...partOf(part.issueSectionTitle)}>
        {t("agent-work.title")}
      </h2>
      {works.map((work) => (
        <WorkPiece
          key={work.id}
          work={work}
          issue={readableId}
          statusChanges={issue.status_changes}
          lacking={lacking}
        />
      ))}
      <Proposals proposals={proposals} agents={names} />
    </section>
  );
}
