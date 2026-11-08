import type { Schema } from "@bugaboo/api-client";
import { partOf, part } from "@bugaboo/ui";

import { AttemptLines } from "./AttemptLines.tsx";
import type { BriefLack, Work } from "./agent-work.ts";
import { Questions } from "./QuestionAnswer.tsx";
import { WorkActions } from "./WorkActions.tsx";
import { WorkBrief } from "./WorkBrief.tsx";
import { WorkFoot } from "./WorkFoot.tsx";
import { WorkHead } from "./WorkHead.tsx";

/**
 * One piece of agent work on the issue (UX §4.17): who works and where it stands, the brief, each
 * attempt, the questions answered right there, *Approve plan*, the review, and why the agent gave
 * up. It is a record of its own, apart from the comments.
 */
export function WorkPiece({
  work,
  issue,
  statusChanges,
  lacking,
  isNewest,
}: {
  readonly work: Work;
  /** The parts of the brief the issue lacks now. */
  readonly lacking: readonly BriefLack[];
  readonly issue: string;
  /** Whether it is the newest work of the issue. */
  readonly isNewest: boolean;
  readonly statusChanges: readonly Schema<"StatusChangeRef">[] | null | undefined;
}) {
  return (
    <div {...partOf(part.agentWorkPiece)}>
      <WorkHead work={work} />
      <WorkBrief work={work} lacking={lacking} />
      {work.attempts.map((attempt) => (
        <AttemptLines key={attempt.id} work={work} attempt={attempt} />
      ))}
      <Questions work={work} />
      <WorkActions work={work} issue={issue} statusChanges={statusChanges} />
      <WorkFoot work={work} issue={issue} isNewest={isNewest} />
    </div>
  );
}
